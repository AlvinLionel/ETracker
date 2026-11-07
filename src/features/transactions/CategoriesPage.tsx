import { useCallback, useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type Category = { id: string; name: string; category_type: "income" | "expense"; is_system: boolean };

export default function CategoriesPage() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [name, setName] = useState("");
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

  async function createCategory(event: FormEvent) {
    event.preventDefault();
    setError("");
    setMessage("");
    const normalizedName = name.trim();
    if (!normalizedName) { setError("Enter a category name."); return; }
    setBusy(true);
    try {
      await invoke("create_category", { name: normalizedName, categoryType });
      setName("");
      setMessage(`${categoryType === "expense" ? "Expense" : "Income"} category created.`);
      await refresh();
    } catch (reason) {
      setError(typeof reason === "string" ? reason : "Category could not be created.");
    } finally { setBusy(false); }
  }

  async function archiveCategory(category: Category) {
    if (category.is_system || !window.confirm(`Archive ${category.name}? Existing history keeps its category; future records can no longer use it.`)) return;
    setError("");
    setMessage("");
    try {
      await invoke("archive_category", { id: category.id });
      setMessage(`${category.name} archived. Existing transaction history is preserved.`);
      await refresh();
    } catch (reason) {
      setError(typeof reason === "string" ? reason : "Category could not be archived.");
    }
  }

  return <section className="mx-auto max-w-5xl space-y-6">
    <header><p className="workspace-accent text-xs font-semibold uppercase tracking-[0.14em]">Finance</p><h1 className="mt-2 text-2xl font-semibold tracking-tight">Categories</h1><p className="workspace-muted mt-2 max-w-3xl text-sm">Create the income and expense categories you want to use while reviewing imported statements, recording transactions and planning your budget. Archiving a category preserves its existing history.</p></header>
    {error && <p role="alert" className="rounded-lg bg-red-50 p-3 text-sm text-red-800">{error}</p>}
    {message && <p role="status" className="rounded-lg bg-emerald-50 p-3 text-sm text-emerald-800">{message}</p>}
    <form onSubmit={createCategory} className="workspace-panel grid gap-4 rounded-xl p-5 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
      <label className="text-sm font-medium">Category name<input aria-label="Category name" required maxLength={100} value={name} onChange={(event) => setName(event.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2" /></label>
      <label className="text-sm font-medium">Category type<select aria-label="Category type" value={categoryType} onChange={(event) => setCategoryType(event.target.value as Category["category_type"])} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="expense">Expense</option><option value="income">Income</option></select></label>
      <button disabled={busy} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50">{busy ? "Saving…" : "Add category"}</button>
    </form>
    <section className="workspace-panel rounded-xl p-5 sm:p-6">
      <h2 className="mb-4 font-semibold">Active categories</h2>
      {categories.length ? <ul className="divide-y border-y border-slate-200">{categories.map((category) => <li key={category.id} className="workspace-row flex flex-wrap items-center justify-between gap-3 py-3 text-sm"><span className="font-medium">{category.name}</span><span className="rounded-full bg-slate-100 px-2.5 py-1 text-xs capitalize text-slate-600">{category.category_type}</span><button type="button" disabled={category.is_system} onClick={() => void archiveCategory(category)} className="ml-auto text-rose-700 disabled:cursor-not-allowed disabled:opacity-40">Archive</button></li>)}</ul> : <p className="workspace-muted rounded-lg border border-dashed border-slate-300 p-6 text-center text-sm">No categories yet. Add categories to group statement activity and make your plans more useful.</p>}
    </section>
  </section>;
}
