import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listActiveCategories } from "../transactions/api/transactions";

type Category = { id: string; name: string; category_type: string };
type Template = { id: string; name: string; currency_code: string };
type Period = { id: string; name: string; currency_code: string; starts_on: string; ends_on: string };
type Line = { id: string; item_type: string; category_id: string | null; label: string; planned_minor: number; actual_minor: number };
type Draft = { item_type: "income" | "expense" | "savings"; category_id: string; label: string; amount: string };
const emptyDraft = (): Draft => ({ item_type: "expense", category_id: "", label: "", amount: "" });
const money = (minor: number, currency: string) => new Intl.NumberFormat(undefined, { style: "currency", currency }).format(minor / 100);

export default function BudgetingPage() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [periods, setPeriods] = useState<Period[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [lines, setLines] = useState<Line[]>([]);
  const [templateName, setTemplateName] = useState("");
  const [drafts, setDrafts] = useState<Draft[]>([emptyDraft()]);
  const [templateId, setTemplateId] = useState("");
  const [month, setMonth] = useState(new Date().toISOString().slice(0, 7));
  const [selectedPeriod, setSelectedPeriod] = useState("");
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const currency = periods.find((period) => period.id === selectedPeriod)?.currency_code ?? "KES";
  const totals = useMemo(() => lines.reduce((result, line) => { result[line.item_type] = (result[line.item_type] ?? 0) + line.planned_minor; if (line.item_type === "expense") result.actual = (result.actual ?? 0) + line.actual_minor; return result; }, {} as Record<string, number>), [lines]);
  const plannedRemaining = (totals.income ?? 0) - (totals.expense ?? 0) - (totals.savings ?? 0);

  async function refresh() {
    const [nextTemplates, nextPeriods] = await Promise.all([invoke<Template[]>("list_budget_templates"), invoke<Period[]>("list_budget_periods")]);
    setTemplates(nextTemplates); setPeriods(nextPeriods);
    if (!templateId && nextTemplates[0]) setTemplateId(nextTemplates[0].id);
    if (!selectedPeriod && nextPeriods[0]) setSelectedPeriod(nextPeriods[0].id);
  }
  async function loadPeriod(id: string) { setSelectedPeriod(id); setLines(id ? await invoke<Line[]>("get_budget_period_items", { budgetId: id }) : []); }
  useEffect(() => { Promise.all([listActiveCategories("income"), listActiveCategories("expense")]).then(([income, expense]) => setCategories([...income, ...expense])); refresh().catch(() => setError("Budget plans could not be loaded.")); }, []);
  useEffect(() => { if (selectedPeriod) invoke<Line[]>("get_budget_period_items", { budgetId: selectedPeriod }).then(setLines).catch(() => setError("Budget details could not be loaded.")); }, [selectedPeriod]);

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
      const id = await invoke<string>("create_budget_template", { request: { name: templateName, currencyCode: "KES", items } });
      setTemplateName(""); setDrafts([emptyDraft()]); await refresh(); setTemplateId(id); setMessage("Budget template saved.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
  }
  async function createPeriod(event: React.FormEvent) {
    event.preventDefault(); setError(""); setMessage("");
    try { const id = await invoke<string>("create_budget_period", { templateId, month }); await refresh(); await loadPeriod(id); setMessage("Monthly budget created as an independent plan."); }
    catch (reason) { setError(typeof reason === "string" ? reason : "The monthly budget could not be created."); }
  }

  return <section className="mx-auto max-w-6xl space-y-7">
    <div><p className="text-xs font-semibold uppercase tracking-widest text-emerald-700">Planning</p><h1 className="mt-2 text-2xl font-semibold">Budgeting</h1><p className="mt-1 text-sm text-slate-600">Plan income, spending and savings, then compare category spending with posted transactions.</p></div>
    {error && <div role="alert" className="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">{error}</div>}{message && <p role="status" className="text-sm text-emerald-800">{message}</p>}
    <div className="grid gap-6 lg:grid-cols-2">
      <form onSubmit={saveTemplate} className="space-y-4 rounded-xl border border-slate-200 bg-white p-5">
        <div><h2 className="font-semibold">Create a reusable template</h2><p className="mt-1 text-sm text-slate-500">New months copy these values. Later changes stay separate.</p></div>
        <label className="block text-sm font-medium">Template name<input required value={templateName} onChange={(e) => setTemplateName(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2" /></label>
        {drafts.map((draft, index) => <div key={index} className="grid gap-2 rounded-lg bg-slate-50 p-3 sm:grid-cols-2">
          <select aria-label={`Planned item type ${index + 1}`} value={draft.item_type} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, item_type: e.target.value as Draft["item_type"], category_id: "" } : item))} className="rounded-md border border-slate-300 bg-white px-2 py-2"><option value="expense">Expense</option><option value="income">Income</option><option value="savings">Intended savings</option></select>
          {draft.item_type !== "savings" && <select aria-label={`Category ${index + 1}`} required value={draft.category_id} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, category_id: e.target.value } : item))} className="rounded-md border border-slate-300 bg-white px-2 py-2"><option value="">Select category</option>{categories.filter((category) => category.category_type === draft.item_type).map((category) => <option key={category.id} value={category.id}>{category.name}</option>)}</select>}
          <input aria-label={`Item label ${index + 1}`} required placeholder={draft.item_type === "savings" ? "Savings goal or purpose" : "Plan label"} value={draft.label} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, label: e.target.value } : item))} className="rounded-md border border-slate-300 px-3 py-2" />
          <input aria-label={`Planned amount ${index + 1}`} required inputMode="decimal" placeholder="Amount (KES)" value={draft.amount} onChange={(e) => setDrafts((current) => current.map((item, i) => i === index ? { ...item, amount: e.target.value } : item))} className="rounded-md border border-slate-300 px-3 py-2" />
        </div>)}
        <div className="flex flex-wrap gap-2"><button type="button" onClick={() => setDrafts((current) => [...current, emptyDraft()])} className="rounded-md border border-slate-300 px-3 py-2 text-sm">Add planned item</button><button className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white">Save template</button></div>
      </form>
      <form onSubmit={createPeriod} className="space-y-4 rounded-xl border border-slate-200 bg-white p-5">
        <div><h2 className="font-semibold">Create a monthly budget</h2><p className="mt-1 text-sm text-slate-500">Each month is a separate snapshot and does not change with its template.</p></div>
        <label className="block text-sm font-medium">Template<select required value={templateId} onChange={(e) => setTemplateId(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="">Choose a template</option>{templates.map((template) => <option key={template.id} value={template.id}>{template.name} · {template.currency_code}</option>)}</select></label>
        <label className="block text-sm font-medium">Month<input type="month" required value={month} onChange={(e) => setMonth(e.target.value)} className="mt-1 w-full rounded-md border border-slate-300 px-3 py-2" /></label>
        <button disabled={!templateId} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">Create monthly budget</button>
      </form>
    </div>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><div className="flex flex-wrap items-end justify-between gap-4"><div><h2 className="font-semibold">Budget performance</h2><p className="mt-1 text-sm text-slate-500">Actual expense is calculated from posted, non-voided category allocations.</p></div><label className="text-sm font-medium">Budget period<select aria-label="Budget period" value={selectedPeriod} onChange={(e) => loadPeriod(e.target.value).catch(() => setError("Budget details could not be loaded."))} className="mt-1 block min-w-64 rounded-md border border-slate-300 bg-white px-3 py-2"><option value="">Choose a budget</option>{periods.map((period) => <option key={period.id} value={period.id}>{period.name}</option>)}</select></label></div>
      {selectedPeriod && <><div className="mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-5"><Summary label="Planned income" amount={totals.income ?? 0} currency={currency}/><Summary label="Planned expenses" amount={totals.expense ?? 0} currency={currency}/><Summary label="Actual expenses" amount={totals.actual ?? 0} currency={currency}/><Summary label="Intended savings" amount={totals.savings ?? 0} currency={currency}/><Summary label="Planned remaining" amount={plannedRemaining} currency={currency}/></div><div className="mt-5 overflow-x-auto"><table className="w-full text-left text-sm"><thead className="border-b text-xs uppercase text-slate-500"><tr><th className="py-2">Plan</th><th>Type</th><th className="text-right">Planned</th><th className="text-right">Actual spending</th><th className="text-right">Remaining</th></tr></thead><tbody>{lines.map((line) => <tr key={line.id} className="border-b border-slate-100"><td className="py-3">{line.label}</td><td className="capitalize">{line.item_type}</td><td className="text-right">{money(line.planned_minor, currency)}</td><td className="text-right">{line.item_type === "expense" ? money(line.actual_minor, currency) : "—"}</td><td className="text-right">{line.item_type === "expense" ? money(line.planned_minor-line.actual_minor, currency) : "—"}</td></tr>)}</tbody></table></div></>}
      {!selectedPeriod && <p className="mt-5 text-sm text-slate-500">Create a monthly budget to review its plan and actual spending.</p>}
    </section>
  </section>;
}

function Summary({ label, amount, currency }: { label: string; amount: number; currency: string }) { return <div className="rounded-lg bg-slate-50 px-4 py-3"><p className="text-xs text-slate-500">{label}</p><p className="mt-1 font-semibold">{money(amount, currency)}</p></div>; }
