import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import UpdateNotice from "./UpdateNotice";

const mocks = vi.hoisted(() => ({ check: vi.fn(), isTauri: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: mocks.isTauri }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: mocks.check }));

describe("UpdateNotice", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.isTauri.mockReturnValue(true);
    mocks.check.mockResolvedValue(null);
  });

  it("quietly skips update checks in the browser and when no update exists", async () => {
    mocks.isTauri.mockReturnValue(false);
    const { unmount } = render(<UpdateNotice />);
    expect(mocks.check).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("Application update")).not.toBeInTheDocument();

    unmount();
    mocks.isTauri.mockReturnValue(true);
    render(<UpdateNotice />);
    await waitFor(() => expect(mocks.check).toHaveBeenCalledWith({ timeout: 10_000 }));
    expect(screen.queryByLabelText("Application update")).not.toBeInTheDocument();
  });

  it("offers an explicit install action and lets the user defer", async () => {
    const update = { version: "0.2.0", downloadAndInstall: vi.fn(), close: vi.fn().mockResolvedValue(undefined) };
    mocks.check.mockResolvedValue(update);
    render(<UpdateNotice />);

    expect(await screen.findByText("ETracker 0.2.0 is ready.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Remind me later" }));
    expect(update.close).toHaveBeenCalledOnce();
    expect(screen.queryByLabelText("Application update")).not.toBeInTheDocument();
  });

  it("shows a retry message when the signed update cannot be installed", async () => {
    const update = { version: "0.2.0", downloadAndInstall: vi.fn().mockRejectedValue(new Error("network")), close: vi.fn() };
    mocks.check.mockResolvedValue(update);
    render(<UpdateNotice />);

    fireEvent.click(await screen.findByRole("button", { name: "Install and restart" }));
    expect(await screen.findByText("The update could not be installed. Check your connection and try again.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Install and restart" })).toBeInTheDocument();
  });
});
