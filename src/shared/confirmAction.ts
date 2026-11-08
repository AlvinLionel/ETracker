import { confirm as nativeConfirm } from "@tauri-apps/plugin-dialog";

export async function confirmAction(message: string): Promise<boolean> {
  if (import.meta.env.MODE === "e2e") return true;
  return nativeConfirm(message, { title: "ETracker", kind: "warning" });
}
