import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import TransactionsPage from "./TransactionsPage";

const mocks=vi.hoisted(()=>({accounts:vi.fn(),categories:vi.fn(),create:vi.fn(),list:vi.fn()}));
vi.mock("../accounts/api/accounts",()=>({listActiveAccounts:mocks.accounts}));
vi.mock("./api/transactions",()=>({createTransaction:mocks.create,listTransactions:mocks.list,listActiveCategories:mocks.categories,updateTransaction:vi.fn(),getTransactionAllocations:vi.fn(),getTransactionAuditHistory:vi.fn(),voidTransaction:vi.fn(),reverseTransaction:vi.fn(),createTransfer:vi.fn()}));

describe("TransactionsPage",()=>{
 beforeEach(()=>{vi.clearAllMocks();mocks.accounts.mockResolvedValue([{id:"a",name:"Main",currency_code:"KES"}]);mocks.categories.mockResolvedValue([{id:"food",name:"Food",category_type:"expense"},{id:"home",name:"Home",category_type:"expense"}]);mocks.list.mockResolvedValue([]);mocks.create.mockResolvedValue("tx");});
 it("allows an uncategorized expense",async()=>{
  render(<TransactionsPage/>); await screen.findByRole("option",{name:"Main (KES)"});
  fireEvent.change(screen.getAllByLabelText("Account",{selector:"select"})[0],{target:{value:"a"}});
  fireEvent.change(screen.getByLabelText("Amount",{selector:"input"}),{target:{value:"12.50"}});
  fireEvent.click(screen.getByRole("button",{name:"Save transaction"}));
  await waitFor(()=>expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({amount_minor:1250,allocations:[]})));
 });
 it("requires split allocations to reconcile before submitting",async()=>{
  render(<TransactionsPage/>); await screen.findByRole("option",{name:"Main (KES)"});
  fireEvent.change(screen.getAllByLabelText("Account",{selector:"select"})[0],{target:{value:"a"}});
  fireEvent.change(screen.getByLabelText("Amount",{selector:"input"}),{target:{value:"12.00"}});
  fireEvent.click(screen.getByRole("button",{name:"Add allocation"}));
  fireEvent.change(screen.getByLabelText("Category"),{target:{value:"food"}});
  fireEvent.change(screen.getByLabelText("Allocation amount"),{target:{value:"10.00"}});
  fireEvent.click(screen.getByRole("button",{name:"Save transaction"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("add up to the full amount");
  expect(mocks.create).not.toHaveBeenCalled();
 });
});
