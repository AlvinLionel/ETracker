
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import AccountForm from "./AccountForm"
import type { CreateAccountInput } from "./types";

describe("AccountForm", () => {
  it("creates an account without an opening balance", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);

    render(
      <AccountForm
        isSubmitting={false}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText(/Account name/), {
      target: { value: "Personal M-Pesa" },
    });

    fireEvent.change(screen.getByLabelText(/Institution/), {
      target: { value: "Safaricom" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(onSubmit).toHaveBeenCalledWith({
      name: "Personal M-Pesa",
      account_type: "mobile_money",
      institution_name: "Safaricom",
      notes: null,
      opening_balance: null,
    } satisfies CreateAccountInput);
  });

  it("requires an effective date when an opening balance is provided", () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);

    render(
      <AccountForm
        isSubmitting={false}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText(/Account name/), {
      target: { value: "Equity Personal" },
    });
    fireEvent.change(screen.getByLabelText(/Amount \(KES\)/), {
      target: { value: "1250.50" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Select the effective date and time",
    );

    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("converts the opening balance to minor units and normalizes its date", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);

    render(
      <AccountForm
        isSubmitting={false}
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText(/Account name/), {
      target: { value: "Equity Personal" },
    });
    fireEvent.change(screen.getByLabelText(/Account type/), {
      target: { value: "bank" },
    });
    fireEvent.change(screen.getByLabelText(/Institution/), {
      target: { value: "Equity Bank" },
    });
    fireEvent.change(screen.getByLabelText(/Amount \(KES\)/), {
      target: { value: "1250.50" },
    });
    fireEvent.change(
      screen.getByLabelText(/Effective date and time/),
      {
        target: { value: "2026-11-05T10:30" },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(onSubmit).toHaveBeenCalledWith({
      name: "Equity Personal",
      account_type: "bank",
      institution_name: "Equity Bank",
      notes: null,
      opening_balance: {
        amount_minor: 125050,
        occurred_at: new Date("2026-11-05T10:30").toISOString(),
      },
    } satisfies CreateAccountInput);
  });
});
