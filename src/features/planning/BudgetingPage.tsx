import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listActiveCategories } from "../transactions/api/transactions";

type Category = { id: string; name: string; category_type: string };
type Template = { id: string; name: string; currency_code: string };
type Period = { id: string; name: string; currency_code: string; starts_on: string; ends_on: string };
type Line = { id: string; item_type: string; category_id: string | null; label: string; planned_minor: number; actual_minor: number };
type Performance = { actual_income_minor: number; actual_expense_minor: number; actual_savings_minor: number; unallocated_minor: number };
type Draft = { item_type: "income" | "expense" | "savings"; category_id: string; label: string; amount: string; actual_minor?: number };
const emptyDraft = (): Draft => ({ item_type: "expense", category_id: "", label: "", amount: "" });
const money = (minor: number, currency: string) => new Intl.NumberFormat(undefined, { style: "currency", currency }).format(minor / 100);

export default function BudgetingPage() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [periods, setPeriods] = useState<Period[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [lines, setLines] = useState<Line[]>([]);
  const [performance, setPerformance] = useState<Performance | null>(null);
  const [templateName, setTemplateName] = useState("");
  const [drafts, setDrafts] = useState<Draft[]>([emptyDraft()]);
  const [templateId, setTemplateId] = useState("");
  const [editingTemplate, setEditingTemplate] = useState(false);
  const [month, setMonth] = useState(new Date().toISOString().slice(0, 7));
  const [selectedPeriod, setSelectedPeriod] = useState("");
  const [editingPeriod, setEditingPeriod] = useState(false);
  const [periodDrafts, setPeriodDrafts] = useState<Draft[]>([]);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const currency = periods.find((period) => period.id === selectedPeriod)?.currency_code ?? "KES";
  const totals = useMemo(() => lines.reduce((result, line) => { result[line.item_type] = (result[line.item_type] ?? 0) + line.planned_minor; if (line.item_type === "expense") result.actual = (result.actual ?? 0) + line.actual_minor; return result; }, {} as Record<string, number>), [lines]);
  const plannedRemaining = (totals.income ?? 0) - (totals.expense ?? 0) - (totals.savings ?? 0);
  const tableLines = editingPeriod
    ? periodDrafts.map((line, index) => ({ id: `edit-${index}`, item_type: line.item_type, category_id: line.category_id || null, label: line.label, amount: line.amount, planned_minor: Math.round(Number(line.amount || "0") * 100), actual_minor: line.actual_minor ?? 0 }))
    : lines.map((line) => ({ ...line, amount: (line.planned_minor / 100).toFixed(2) }));

  async function refresh() {
    const [nextTemplates, nextPeriods] = await Promise.all([invoke<Template[]>("list_budget_templates"), invoke<Period[]>("list_budget_periods")]);
    setTemplates(nextTemplates); setPeriods(nextPeriods);
    if (!templateId && nextTemplates[0]) setTemplateId(nextTemplates[0].id);
    if (!selectedPeriod && nextPeriods[0]) setSelectedPeriod(nextPeriods[0].id);
  }
  async function loadPeriod(id: string) { setSelectedPeriod(id); if (!id) { setLines([]); setPerformance(null); return; } const [nextLines, totals] = await Promise.all([invoke<Line[]>("get_budget_period_items", { budgetId: id }), invoke<Performance>("get_budget_period_totals", { budgetId: id })]); setLines(nextLines); setPerformance(totals); }
  useEffect(() => { Promise.all([listActiveCategories("income"), listActiveCategories("expense")]).then(([income, expense]) => setCategories([...income, ...expense])); refresh().catch(() => setError("Budget plans could not be loaded.")); }, []);
  useEffect(() => { if (selectedPeriod) Promise.all([invoke<Line[]>("get_budget_period_items", { budgetId: selectedPeriod }), invoke<Performance>("get_budget_period_totals", { budgetId: selectedPeriod })]).then(([nextLines, totals]) => { setLines(nextLines); setPerformance(totals); }).catch(() => setError("Budget details could not be loaded.")); }, [selectedPeriod]);

  async function saveTemplate(event: React.FormEvent) {
    event.preventDefault(); setError(""); setMessage("");
    try {
      const items = drafts.map((draft) => {
        const amount = draft.amount.trim();
        if (!/^\d+(?:\.\d{1,2})?$/.test(amount)) throw new Error("Enter a valid positive amount for each planned item.");
        const [whole, fraction = ""] = amount.split(".");
        const amount_minor = Number(BigInt(whole) * 100n + BigInt((fraction + "00").slice(0, 2)));
        return { item_type: draft.item_type, category_id: draft.item_type === "savings" ? null : draft.category_id || null, label: draft.label.trim(), amount_minor };
      });
      if (editingTemplate) {
        await invoke("update_budget_template", { templateId, name: templateName, items });
        setMessage("Template updated. Existing monthly budgets retain their copied values.");
      } else {
        const id = await invoke<string>("create_budget_template", { request: { name: templateName, currency_code: "KES", items } });
        setTemplateId(id); setMessage("Budget template saved.");
      }
      setTemplateName(""); setDrafts([emptyDraft()]); setEditingTemplate(false); await refresh();
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
  }
  async function beginTemplateEdit() {
    if (!templateId) return;
    try {
      const template = templates.find((item) => item.id === templateId);
      const items = await invoke<{ item_type: Draft["item_type"]; category_id: string | null; label: string; amount_minor: number }[]>("list_budget_template_items", { templateId });
      setTemplateName(template?.name ?? "");
      setDrafts(items.map((item) => ({ item_type: item.item_type, category_id: item.category_id ?? "", label: item.label, amount: (item.amount_minor / 100).toFixed(2) })));
      setEditingTemplate(true); setError("");
    } catch { setError("The selected template could not be loaded for editing."); }
  }
  async function createPeriod(event: React.FormEvent) {
    event.preventDefault(); setError(""); setMessage("");
    try { const id = await invoke<string>("create_budget_period", { templateId, month }); await refresh(); await loadPeriod(id); setMessage("Monthly budget created as an independent plan."); }
    catch (reason) { setError(typeof reason === "string" ? reason : "The monthly budget could not be created."); }
  }
  function beginPeriodEdit() { setPeriodDrafts(lines.map((line) => ({ item_type: line.item_type as Draft["item_type"], category_id: line.category_id ?? "", label: line.label, amount: (line.planned_minor / 100).toFixed(2), actual_minor: line.actual_minor }))); setEditingPeriod(true); }
  async function savePeriodEdit() {
    setError("");
    try {
      const items = periodDrafts.map((draft) => {
        const amount = draft.amount.trim();
        if (!/^\d+(?:\.\d{1,2})?$/.test(amount)) throw new Error("Enter a valid positive amount for each planned item.");
        const [whole, fraction = ""] = amount.split(".");
        const amount_minor = Number(BigInt(whole) * 100n + BigInt((fraction + "00").slice(0, 2)));
        return { item_type: draft.item_type, category_id: draft.item_type === "savings" ? null : draft.category_id || null, label: draft.label.trim(), amount_minor };
      });
      await invoke("update_budget_period_items", { budgetId: selectedPeriod, items });
      setEditingPeriod(false); await loadPeriod(selectedPeriod); setMessage("Monthly plan updated independently.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : typeof reason === "string" ? reason : "The monthly plan could not be saved."); }
  }

  return <section className="mx-auto max-w-6xl space-y-7">
    <div><p className="text-xs font-semibold uppercase tracking-widest text-emerald-700">Planning</p><h1 className="mt-2 text-2xl font-semibold">Budgeting</h1><p className="mt-1 text-sm text-slate-600">Plan income, spending and savings, then compare category spending with posted transactions.</p></div>
    {error && <div role="alert" className="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">{error}</div>}{message && <p role="status" className="text-sm text-emerald-800">{message}</p>}
    <div className="grid gap-6 lg:grid-cols-2">
      <form onSubmit={saveTemplate} className="space-y-4 rounded-xl border border-slate-200 bg-white p-5">
        <div className="flex flex-wrap items-center justify-between gap-2"><div><h2 className="font-semibold">{editingTemplate ? "Edit reusable template" : "Create a reusable template"}</h2><p className="mt-1 text-sm text-slate-500">New months copy these values. Later changes stay separate.</p></div><button type="button" disabled={!templateId} onClick={() => beginTemplateEdit().catch(() => setError("The selected template could not be loaded for editing."))} className="rounded-md border border-slate-300 px-3 py-2 text-xs disabled:opacity-50">Edit selected template</button></div>
        <label className="block text-sm font-medium">Template to edit or use for a month<select aria-label="Template to edit or use for a month" value={templateId} onChange={(e) => { setTemplateId(e.target.value); setEditingTemplate(false); }} className="mt-1 w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="">Choose a template</option>{templates.map((template) => <option key={template.id} value={template.id}>{template.name} · {template.currency_code}</option>)}</select></label>
        <label className="block text-sm font-medium">Template name<input required value={templateName} onChange={(e) => setTemplateName(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2" /></label>
        {drafts.map((draft, index) => <div key={index} className="grid gap-2 rounded-lg bg-slate-50 p-3 sm:grid-cols-2">
          <select aria-label={`Planned item type ${index + 1}`} value={draft.item_type} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, item_type: e.target.value as Draft["item_type"], category_id: "" } : item))} className="rounded-md border border-slate-300 bg-white px-2 py-2"><option value="expense">Expense</option><option value="income">Income</option><option value="savings">Intended savings</option></select>
          {draft.item_type !== "savings" && <select aria-label={`Category ${index + 1}`} required value={draft.category_id} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, category_id: e.target.value } : item))} className="rounded-md border border-slate-300 bg-white px-2 py-2"><option value="">Select category</option>{categories.filter((category) => category.category_type === draft.item_type).map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}</select>}
          <input aria-label={`Item label ${index + 1}`} required placeholder={draft.item_type === "savings" ? "Savings goal or purpose" : "Plan label"} value={draft.label} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, label: e.target.value } : item))} className="rounded-md border border-slate-300 px-3 py-2" />
          <input aria-label={`Planned amount ${index + 1}`} required inputMode="decimal" placeholder="Amount (KES)" value={draft.amount} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, amount: e.target.value } : item))} className="rounded-md border border-slate-300 px-3 py-2" />
        </div>)}
        <div className="flex flex-wrap gap-2"><button type="button" onClick={() => setDrafts((current) => [...current, emptyDraft()])} className="rounded-md border border-slate-300 px-3 py-2 text-sm">Add planned item</button><button className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white">{editingTemplate ? "Save template changes" : "Save template"}</button>{editingTemplate && <button type="button" onClick={() => { setEditingTemplate(false); setTemplateName(""); setDrafts([emptyDraft()]); }} className="rounded-md border border-slate-300 px-3 py-2 text-sm">Cancel edit</button>}</div>
      </form>
      <form onSubmit={createPeriod} className="space-y-4 rounded-xl border border-slate-200 bg-white p-5">
        <div><h2 className="font-semibold">Create a monthly budget</h2><p className="mt-1 text-sm text-slate-500">Each month is a separate snapshot and does not change with its template.</p></div>
        <label className="block text-sm font-medium">Template<select required value={templateId} onChange={(e) => setTemplateId(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="">Choose a template</option>{templates.map((template) => <option key={template.id} value={template.id}>{template.name} · {template.currency_code}</option>)}</select></label>
        <label className="block text-sm font-medium">Month<input type="month" required value={month} onChange={(e) => setMonth(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2" /></label>
        <button disabled={!templateId} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">Create monthly budget</button>
      </form>
    </div>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><div className="flex flex-wrap items-end justify-between gap-4"><div><h2 className="font-semibold">Budget performance</h2><p className="mt-1 text-sm text-slate-500">Actual expense is calculated from posted, non-voided category allocations.</p></div><label className="text-sm font-medium">Budget period<select aria-label="Budget period" value={selectedPeriod} onChange={(e) => loadPeriod(e.target.value).catch(() => setError("Budget details could not be loaded."))} className="mt-1 block min-w-64 rounded-md border border-slate-300 bg-white px-3 py-2"><option value="">Choose a budget</option>{periods.map((period) => <option key={period.id} value={period.id}>{period.name}</option>)}</select></label></div>
      {selectedPeriod && <><div className="mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4"><Summary label="Planned income" amount={totals.income ?? 0} currency={currency}/><Summary label="Actual income" amount={performance?.actual_income_minor ?? 0} currency={currency}/><Summary label="Planned expenses" amount={totals.expense ?? 0} currency={currency}/><Summary label="Actual expenses" amount={performance?.actual_expense_minor ?? totals.actual ?? 0} currency={currency}/><Summary label="Intended savings" amount={totals.savings ?? 0} currency={currency}/><Summary label="Actual savings" amount={performance?.actual_savings_minor ?? 0} currency={currency}/><Summary label="Planned remaining" amount={plannedRemaining} currency={currency}/><Summary label="Unallocated actual" amount={performance?.unallocated_minor ?? 0} currency={currency}/></div><div className="mt-5 flex flex-wrap justify-end gap-2">{editingPeriod ? <><button type="button" onClick={() => setPeriodDrafts((current) => [...current, emptyDraft()])} className="rounded-md border border-slate-300 px-3 py-2 text-sm">Add planned item</button><button type="button" onClick={() => savePeriodEdit().catch(() => setError("The monthly plan could not be saved."))} className="rounded-md bg-emerald-700 px-3 py-2 text-sm font-medium text-white">Save monthly plan</button><button type="button" onClick={() => setEditingPeriod(false)} className="rounded-md border border-slate-300 px-3 py-2 text-sm">Cancel</button></> : <button type="button" disabled={!lines.length} onClick={beginPeriodEdit} className="rounded-md border border-slate-300 px-3 py-2 text-sm disabled:opacity-50">Edit this monthly plan</button>}</div><div className="mt-2 overflow-x-auto"><table className="w-full text-left text-sm"><thead className="border-b text-xs uppercase text-slate-500"><tr><th className="py-2">Plan</th><th>Type</th><th className="text-right">Planned</th><th className="text-right">Actual</th><th className="text-right">Variance</th>{editingPeriod && <th />}</tr></thead><tbody>{tableLines.map((line, index) => { const actual = line.actual_minor; const planned = line.planned_minor; const isEditing = editingPeriod; return <tr key={line.id} className="border-b border-slate-100"><td className="py-3">{isEditing ? <div className="flex flex-col gap-2"><input aria-label={`Plan label ${index + 1}`} value={line.label} onChange={(e) => setPeriodDrafts((current) => current.map((item, i) => i === index ? { ...item, label: e.target.value } : item))} className="rounded-md border border-slate-300 px-2 py-1" />{line.item_type !== "savings" && <select aria-label={`Plan category ${index + 1}`} value={line.category_id ?? ""} onChange={(e) => setPeriodDrafts((current) => current.map((item, i) => i === index ? { ...item, category_id: e.target.value } : item))} className="rounded-md border border-slate-300 bg-white px-2 py-1">{categories.filter((category) => category.category_type === line.item_type).map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}</select>}</div> : line.label}</td><td>{isEditing ? <select aria-label={`Plan type ${index + 1}`} value={line.item_type} onChange={(e) => setPeriodDrafts((current) => current.map((item, i) => i === index ? { ...item, item_type: e.target.value as Draft["item_type"], category_id: "" } : item))} className="rounded-md border border-slate-300 bg-white px-2 py-1"><option value="expense">Expense</option><option value="income">Income</option><option value="savings">Savings</option></select> : <span className="capitalize">{line.item_type}</span>}</td><td className="text-right">{isEditing ? <input aria-label={`Plan amount ${index + 1}`} inputMode="decimal" value={line.amount} onChange={(e) => setPeriodDrafts((current) => current.map((item, i) => i === index ? { ...item, amount: e.target.value } : item))} className="w-28 rounded-md border border-slate-300 px-2 py-1 text-right" /> : money(line.planned_minor, currency)}</td><td className="text-right">{line.item_type === "expense" || line.item_type === "income" ? money(actual, currency) : "—"}</td><td className="text-right">{line.item_type === "expense" ? money(planned-actual, currency) : "—"}</td>{isEditing && <td><button type="button" aria-label={`Remove plan item ${index + 1}`} onClick={() => setPeriodDrafts((current) => current.filter((_, i) => i !== index))} className="text-xs text-red-700">Remove</button></td>}</tr>; })}</tbody></table></div></>}
      {!selectedPeriod && <p className="mt-5 text-sm text-slate-500">Create a monthly budget to review its plan and actual spending.</p>}
    </section>
  </section>;
}

function Summary({ label, amount, currency }: { label: string; amount: number; currency: string }) { return <div className="rounded-lg bg-slate-50 px-4 py-3"><p className="text-xs text-slate-500">{label}</p><p className="mt-1 font-semibold">{money(amount, currency)}</p></div>; }
