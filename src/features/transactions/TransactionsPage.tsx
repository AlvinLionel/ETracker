import { FormEvent, useCallback, useEffect, useState } from "react";
import { listActiveAccounts } from "../accounts/api/accounts";
import type { Account } from "../accounts/types";
import { createTransaction, listTransactions } from "./api/transactions";
import type { TransactionRecord } from "./types";

function toMinorUnits(value:string):number|null {
  if(!/^\d+(?:\.\d{1,2})?$/.test(value)) return null;
  const [whole,fraction=""]=value.split("."); const minor=BigInt(whole)*100n+BigInt((fraction+"00").slice(0,2));
  return minor>0n&&minor<=BigInt(Number.MAX_SAFE_INTEGER)?Number(minor):null;
}
function formatAmount(record:TransactionRecord){return new Intl.NumberFormat("en-KE",{style:"currency",currency:record.currency_code}).format(record.amount_minor/100)}
function today(){const now=new Date();return `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")}`}

export default function TransactionsPage(){
  const [accounts,setAccounts]=useState<Account[]>([]); const [records,setRecords]=useState<TransactionRecord[]>([]);
  const [accountId,setAccountId]=useState("");const [from,setFrom]=useState("");const [to,setTo]=useState("");
  const [kind,setKind]=useState<"income"|"expense">("expense");const [amount,setAmount]=useState("");const [description,setDescription]=useState("");const [date,setDate]=useState(`${today()}T12:00`);
  const [busy,setBusy]=useState(false);const [error,setError]=useState("");const [notice,setNotice]=useState("");
  const refresh=useCallback(async()=>{setRecords(await listTransactions(accountId,from,to));},[accountId,from,to]);
  useEffect(()=>{listActiveAccounts().then(setAccounts).catch(()=>setError("Accounts could not be loaded."));},[]);
  useEffect(()=>{refresh().catch(()=>setError("Transactions could not be loaded."));},[refresh]);
  async function submit(event:FormEvent){event.preventDefault();setError("");setNotice("");const minor=toMinorUnits(amount);const account=accounts.find(a=>a.id===accountId);if(!account){setError("Choose an active account.");return}if(minor===null){setError("Enter an amount greater than zero with up to two decimal places.");return}
    setBusy(true);try{await createTransaction({account_id:accountId,transaction_type:kind,amount_minor:minor,currency_code:account.currency_code,occurred_at:new Date(date).toISOString(),description:description.trim()||null,notes:null,allocations:[]});setAmount("");setDescription("");setNotice("Transaction saved.");await refresh();}catch(e){setError(typeof e==="string"?e:"Transaction could not be saved.");}finally{setBusy(false)}}
  return <div className="mx-auto max-w-6xl space-y-8">
    <div><p className="text-sm text-slate-500">Finance</p><h1 className="mt-1 text-2xl font-semibold">Transactions</h1><p className="mt-2 text-sm text-slate-600">Record income and expenses. Category assignment can be added after saving.</p></div>
    <form onSubmit={submit} className="grid gap-4 border-y border-slate-200 py-6 md:grid-cols-3">
      <label className="text-sm font-medium">Type<select value={kind} onChange={e=>setKind(e.target.value as "income"|"expense")} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="expense">Expense</option><option value="income">Income</option></select></label>
      <label className="text-sm font-medium">Account<select value={accountId} onChange={e=>setAccountId(e.target.value)} required className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2"><option value="">Choose account</option>{accounts.map(a=><option key={a.id} value={a.id}>{a.name} ({a.currency_code})</option>)}</select></label>
      <label className="text-sm font-medium">Amount<input value={amount} onChange={e=>setAmount(e.target.value)} inputMode="decimal" placeholder="0.00" required className="mt-1 block w-full rounded-md border border-slate-300 px-3 py-2"/></label>
      <label className="text-sm font-medium">Date and time<input type="datetime-local" value={date} onChange={e=>setDate(e.target.value)} required className="mt-1 block w-full rounded-md border border-slate-300 px-3 py-2"/></label>
      <label className="text-sm font-medium md:col-span-2">Description<input value={description} onChange={e=>setDescription(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-3 py-2"/></label>
      <div className="md:col-span-3">{error&&<p role="alert" className="mb-3 text-sm text-rose-700">{error}</p>}{notice&&<p role="status" className="mb-3 text-sm text-emerald-700">{notice}</p>}<button disabled={busy} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50">{busy?"Saving…":"Save transaction"}</button></div>
    </form>
    <section><div className="mb-4 flex flex-wrap items-end gap-3"><div className="mr-auto"><h2 className="text-lg font-semibold">History</h2><p className="text-sm text-slate-500">Posted activity, ordered by effective date.</p></div>
      <label className="text-xs font-medium">Account<select value={accountId} onChange={e=>setAccountId(e.target.value)} className="mt-1 block rounded border border-slate-300 bg-white px-2 py-1"><option value="">All accounts</option>{accounts.map(a=><option key={a.id} value={a.id}>{a.name}</option>)}</select></label>
      <label className="text-xs font-medium">From<input type="date" value={from} onChange={e=>setFrom(e.target.value)} className="mt-1 block rounded border border-slate-300 px-2 py-1"/></label><label className="text-xs font-medium">To<input type="date" value={to} onChange={e=>setTo(e.target.value)} className="mt-1 block rounded border border-slate-300 px-2 py-1"/></label>
    </div><div className="overflow-x-auto border-y border-slate-200"><table className="w-full text-left text-sm"><thead className="text-xs uppercase tracking-wide text-slate-500"><tr><th className="py-3 pr-4">Date</th><th className="py-3 pr-4">Description</th><th className="py-3 pr-4">Account</th><th className="py-3 pr-4">Type</th><th className="py-3 text-right">Amount</th></tr></thead><tbody>{records.map(r=><tr key={r.id} className="border-t border-slate-100"><td className="py-3 pr-4">{new Date(r.occurred_at).toLocaleDateString()}</td><td className="py-3 pr-4">{r.description||"—"}</td><td className="py-3 pr-4">{accounts.find(a=>a.id===r.account_id)?.name||"Unknown account"}</td><td className="py-3 pr-4 capitalize">{r.transaction_type}</td><td className={`py-3 text-right font-medium ${r.transaction_type==="income"?"text-emerald-700":"text-rose-700"}`}>{r.transaction_type==="expense"?"−":"+"}{formatAmount(r)}</td></tr>)}</tbody></table>{records.length===0&&<p className="py-8 text-center text-sm text-slate-500">No transactions match these filters.</p>}</div></section>
  </div>
}
