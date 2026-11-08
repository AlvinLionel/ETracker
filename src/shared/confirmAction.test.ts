import { beforeEach, describe, expect, it, vi } from "vitest";

const nativeConfirm = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: nativeConfirm }));

import { confirmAction } from "./confirmAction";

describe("confirmAction", () => {
  beforeEach(() => nativeConfirm.mockReset());

  it("awaits the native confirmation result and uses a warning prompt", async () => {
    nativeConfirm.mockResolvedValue(false);
    await expect(confirmAction("Archive this category?")).resolves.toBe(false);
    expect(nativeConfirm).toHaveBeenCalledWith("Archive this category?", { title: "ETracker", kind: "warning" });
  });
});
