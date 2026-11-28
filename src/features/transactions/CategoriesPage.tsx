import { useCallback, useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { confirmAction } from "../../shared/confirmAction";

type Category = { id: string; name: string; category_type: "income" | "expense" | "emergency"; is_system: boolean };
type Rule = { category_id: string; category_name: string; category_type: "income" | "expense"; match_text: string };

export default function CategoriesPage() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [name, setName] = useState("");
  const [identityTerms, setIdentityTerms] = useState("");
  const [rules, setRules] = useState<Rule[]>([]);
  const [editing, setEditing] = useState<Category | null>(null);
  const [editingName, setEditingName] = useState("");
  const [editingTerms, setEditingTerms] = useState("");
  const [deleteBlocked, setDeleteBlocked] = useState<Category | null>(null);
  const [categoryType, setCategoryType] = useState<Category["category_type"]>("expense");
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const [income, expenses] = await Promise.all([
      invoke<Category[]>("list_active_categories", { categoryType: "income" }),
      invoke<Category[]>("list_active_categories", { categoryType: "expense" }),
    ]);
    setCategories([...expenses, ...income]);
  }, []);

  useEffect(() => { void refresh().catch(() => setError("Categories could not be loaded.")); }, [refresh]);
  useEffect(() => { invoke<Rule[]>("list_import_category_rules").then(setRules).catch(() => setError("Category identities could not be loaded.")); }, []);

  function terms(value: string) { return value.split(/[\n,;]/).map((item) => item.trim()).filter(Boolean); }

  async function createCategory(event: FormEvent) {
    event.preventDefault();
    setError("");
    setMessage("");
    const normalizedName = name.trim();
    if (!normalizedName) { setError("Enter a category name."); return; }
    const matches = terms(identityTerms);
    if (!matches.length) { setError("Add at least one phrase the system can recognize in statement descriptions."); return; }
    setBusy(true);
    try {
      const category = await invoke<Category>("create_category", { name: normalizedName, categoryType });
      try {
        await invoke("save_import_category_rule", { categoryId: category.id, categoryName: normalizedName, categoryType: categoryType === "income" ? "income" : "expense", matchTexts: matches });
      } catch (reason) {
        await invoke("delete_category", { id: category.id }).catch(() => undefined);
        throw reason;
      }
      setName("");
      setIdentityTerms("");
      setMessage(`${categoryType === "expense" ? "Expense" : categoryType === "emergency" ? "Emergency" : "Income"} category created.`);
      await Promise.all([refresh(), invoke<Rule[]>("list_import_category_rules").then(setRules)]);
    } catch (reason) {
      setError(typeof reason === "string" ? reason : "Category could not be created.");
    } finally { setBusy(false); }
  }

  function beginEdit(category: Category) {
    setEditing(category); setEditingName(category.name);
    setEditingTerms(rules.filter((rule) => rule.category_id === category.id).map((rule) => rule.match_text).join(", "));
  }

  async function saveEdit(event: FormEvent) {
    event.preventDefault(); if (!editing) return;
    setError(""); setMessage(""); setBusy(true);
    const matchType = editing.category_type === "income" ? "income" : "expense";
    try {
      const matches = terms(editingTerms);
      if (!matches.length) { setError("Keep at least one identifying phrase so imports can continue matching this category."); return; }
      await invoke("save_import_category_rule", { categoryId: editing.id, categoryName: editingName.trim(), categoryType: matchType, matchTexts: matches });
      setMessage(`${editing.name} updated. Future imports can identify it using the saved phrases; existing records are unchanged.`); setEditing(null);
      await Promise.all([refresh(), invoke<Rule[]>("list_import_category_rules").then(setRules)]);
    } catch (reason) { setError(typeof reason === "string" ? reason : "Category could not be updated. Check that each identity phrase is unique."); }
    finally { setBusy(false); }
  }

  async function deleteCategory(category: Category) {
    if (category.is_system || !(await confirmAction(`Delete ${category.name}? This works only when no ledger, plan, or import record refers to it.`))) return;
    setError(""); setMessage(""); setDeleteBlocked(null);
    try { await invoke("delete_category", { id: category.id }); setMessage(`${category.name} deleted.`); await Promise.all([refresh(), invoke<Rule[]>("list_import_category_rules").then(setRules)]); }
    catch (reason) { const message = typeof reason === "string" ? reason : "The category could not be deleted."; setError(message); if (message.includes("Archive it")) setDeleteBlocked(category); }
  }

  async function archiveReferencedCategory() {
    if (!deleteBlocked) return;
    try { await invoke("archive_category", { id: deleteBlocked.id }); setMessage(`${deleteBlocked.name} archived. Existing history and plans are preserved.`); setError(""); setDeleteBlocked(null); await refresh(); }
    catch { setError("The category could not be archived."); }
  }

  return <section className="mx-auto max-w-5xl space-y-6">
    <header><p className="workspace-accent text-xs font-semibold uppercase tracking-[0.14em]">Finance</p><h1 className="mt-2 text-2xl font-semibold tracking-tight">Categories</h1><p className="workspace-muted mt-2 max-w-3xl text-sm">Name categories for your records and add identifying phrases that help classify matching statement lines. Edits affect future imports; existing records keep their history.</p></header>
    {error && <div role="alert" className="flex flex-wrap items-center gap-3 rounded-lg bg-red-50 p-3 text-sm text-red-800"><span className="mr-auto">{error}</span>{deleteBlocked && <button type="button" onClick={() => void archiveReferencedCategory()} className="rounded-md border border-rose-300 px-3 py-1.5 font-semibold hover:bg-rose-100">Archive instead</button>}</div>}
    {message && <p role="status" className="rounded-lg bg-emerald-50 p-3 text-sm text-emerald-800">{message}</p>}
    <form onSubmit={createCategory} className="workspace-panel grid gap-4 rounded-xl p-5 sm:grid-cols-2 sm:items-end">
      <label className="text-sm font-medium">Category name<input aria-label="Category name" required maxLength={100} value={name} onChange={(event) => setName(event.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2" /></label>
      <label className="text-sm font-medium">Category type<select aria-label="Category type" value={categoryType} onChange={(event) => setCategoryType(event.target.value as Category["category_type"])} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="expense">Expense</option><option value="income">Income</option><option value="emergency">Emergency</option></select></label>
      <label className="text-sm font-medium sm:col-span-2">Identifying phrases <span className="font-normal text-slate-500">(comma separated; required{categoryType === "emergency" ? ", matched against expense descriptions" : ""})</span><input aria-label="Identifying phrases" required value={identityTerms} onChange={(event) => setIdentityTerms(event.target.value)} placeholder="e.g. supermarket name, grocery" className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2" /></label>
      <button disabled={busy} className="justify-self-start rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50 sm:col-span-2">{busy ? "Saving…" : "Add category"}</button>
    </form>
    <section className="workspace-panel rounded-xl p-5 sm:p-6">
      <h2 className="mb-4 font-semibold">Active categories</h2>
      {categories.length ? <ul className="divide-y border-y border-slate-200">{categories.map((category) => <li key={category.id} className="workspace-row py-3 text-sm"><div className="flex flex-wrap items-center gap-3"><span className="font-medium">{category.name}</span><span className="rounded-full bg-slate-100 px-2.5 py-1 text-xs capitalize text-slate-600">{category.category_type}</span><div className="ml-auto flex gap-2"><button type="button" disabled={category.is_system} onClick={() => beginEdit(category)} className="rounded-md border border-slate-300 px-2.5 py-1.5 text-xs font-semibold hover:bg-slate-100 disabled:opacity-40">Edit</button><button type="button" disabled={category.is_system} onClick={() => void deleteCategory(category)} className="rounded-md border border-rose-200 px-2.5 py-1.5 text-xs font-semibold text-rose-700 hover:bg-rose-50 disabled:opacity-40">Delete</button></div></div>{editing?.id === category.id && <form onSubmit={saveEdit} className="mt-3 grid gap-3 rounded-lg bg-[var(--workspace-subtle-surface)] p-3 sm:grid-cols-2"><label className="text-xs font-semibold">Display name<input aria-label={`Display name for ${category.name}`} required maxLength={100} value={editingName} onChange={(event) => setEditingName(event.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm" /></label><label className="text-xs font-semibold">Identifying phrases<input aria-label={`Identifying phrases for ${category.name}`} value={editingTerms} onChange={(event) => setEditingTerms(event.target.value)} placeholder="Comma separated" className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm" /></label><div className="flex gap-2 sm:col-span-2"><button disabled={busy} className="rounded-md bg-emerald-700 px-3 py-1.5 text-xs font-semibold text-white">Save changes</button><button type="button" onClick={() => setEditing(null)} className="rounded-md border border-slate-300 px-3 py-1.5 text-xs font-semibold">Cancel</button></div></form>}</li>)}</ul> : <p className="workspace-muted rounded-lg border border-dashed border-slate-300 p-6 text-center text-sm">No categories yet. Add categories to group statement activity and make your plans more useful.</p>}
    </section>
  </section>;
}
