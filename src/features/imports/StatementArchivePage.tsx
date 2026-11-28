import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { Download, FileClock, FolderOpen } from "lucide-react";

type RetainedAsset = {
  batch_id: string; account_id: string; account_name: string; source_file_name: string;
  source_format: string; batch_status: string; imported_at: string; retained_until: string; size_bytes: number;
};

const expiryDate = (value: string) => new Date(`${value.replace(" ", "T")}Z`);
const formatDate = (value: string) => new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(expiryDate(value));

export default function StatementArchivePage({ navigate, onReviewBatch }: { navigate?: (page: string) => void; onReviewBatch?: (batchId: string) => void }) {
  const [assets, setAssets] = useState<RetainedAsset[]>([]);
  const [busy, setBusy] = useState(true);
  const [activeDownload, setActiveDownload] = useState("");
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  useEffect(() => {
    let active = true;
    invoke<RetainedAsset[]>("list_retained_import_assets")
      .then((result) => { if (active) setAssets(result); })
      .catch(() => { if (active) setError("Retained statement files could not be loaded."); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, []);

  async function saveOriginal(asset: RetainedAsset) {
    setError(""); setMessage(""); setActiveDownload(asset.batch_id);
    try {
      const bytes = await invoke<number[]>("get_import_source_file", { batchId: asset.batch_id });
      const ext = asset.source_file_name.split(".").pop()?.toLowerCase();
      const path = await saveDialog({ defaultPath: asset.source_file_name, ...(ext ? { filters: [{ name: "Original statement", extensions: [ext] }] } : {}) });
      if (!path) { setMessage("Save cancelled; no file was written."); return; }
      await writeFile(path, new Uint8Array(bytes));
      setMessage(`${asset.source_file_name} was saved to the selected location.`);
    } catch (reason) {
      setError(typeof reason === "string" ? reason : "The original statement could not be saved. It may have reached the end of its retention period.");
    } finally { setActiveDownload(""); }
  }

  return <section className="mx-auto max-w-6xl space-y-5">
    <header className="flex items-end gap-4"><div><p className="workspace-accent font-mono text-xs font-semibold uppercase tracking-[0.1em]">Finance / Statement archive</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Statement archive</h1><p className="workspace-muted mt-2 max-w-3xl text-sm">Find statements used in previous imports, check their retention date, and save an original copy or return to its review workspace.</p></div><FileClock aria-hidden="true" className="mb-1 ml-auto hidden text-[var(--workspace-accent)] sm:block" size={30}/></header>
    <div className="workspace-panel rounded-xl p-4 text-sm"><strong>Original files are kept for 90 days after recent workspace activity.</strong><p className="workspace-muted mt-1">Expired originals are removed automatically. Import records and posted transactions follow their own history rules.</p></div>
    {error && <p role="alert" className="rounded-lg bg-rose-50 p-3 text-sm text-rose-800">{error}</p>}
    {message && <p role="status" className="rounded-lg bg-emerald-50 p-3 text-sm text-emerald-800">{message}</p>}
    {busy ? <p role="status" className="workspace-muted p-6 text-center text-sm">Loading retained statements…</p> : assets.length === 0 ? <div className="workspace-panel rounded-xl p-8 text-center"><FolderOpen aria-hidden="true" className="mx-auto text-[var(--workspace-muted)]"/><h2 className="mt-3 font-semibold">No retained statements</h2><p className="workspace-muted mt-1 text-sm">Original files appear here after you create an import workspace.</p><button type="button" onClick={() => navigate?.("Imports")} className="mt-4 rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white">Import a statement</button></div> : <div className="workspace-panel overflow-hidden rounded-xl"><div className="overflow-x-auto"><table className="w-full min-w-[720px] text-left text-sm"><thead className="workspace-subtle-surface"><tr><th className="p-3 font-semibold">Original statement</th><th className="p-3 font-semibold">Account</th><th className="p-3 font-semibold">Added to workspace</th><th className="p-3 font-semibold">Original kept until</th><th className="p-3 font-semibold">Workspace</th><th className="p-3 font-semibold">Actions</th></tr></thead><tbody className="divide-y divide-[var(--workspace-line)]">{assets.map((asset) => <tr key={asset.batch_id} className="workspace-row"><th scope="row" className="p-3 font-medium"><span className="block max-w-64 break-all">{asset.source_file_name}</span><span className="workspace-muted mt-1 block text-xs">{asset.source_format.toUpperCase()} · {(asset.size_bytes / 1024).toFixed(0)} KB</span></th><td className="p-3">{asset.account_name}</td><td className="p-3 whitespace-nowrap">{formatDate(asset.imported_at)}</td><td className="p-3 whitespace-nowrap">{formatDate(asset.retained_until)}</td><td className="p-3 capitalize">{asset.batch_status.replace(/_/g, " ")}</td><td className="p-3"><div className="flex gap-2"><button type="button" disabled={activeDownload === asset.batch_id} onClick={() => void saveOriginal(asset)} className="inline-flex items-center gap-1.5 rounded-md border border-slate-300 px-2.5 py-1.5 text-xs font-semibold hover:bg-slate-100 disabled:opacity-50"><Download size={14}/>{activeDownload === asset.batch_id ? "Saving…" : "Save original"}</button><button type="button" onClick={() => onReviewBatch ? onReviewBatch(asset.batch_id) : navigate?.("Imports")} className="rounded-md border border-slate-300 px-2.5 py-1.5 text-xs font-semibold hover:bg-slate-100">Review workspace</button></div></td></tr>)}</tbody></table></div></div>}
  </section>;
}
