import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import TransactionsPage from "./TransactionsPage";

const mocks=vi.hoisted(()=>({accounts:vi.fn(),categories:vi.fn(),eligibleRefunds:vi.fn(),create:vi.fn(),list:vi.fn(),allocations:vi.fn(),update:vi.fn(),void:vi.fn(),reverse:vi.fn(),transfer:vi.fn(),audit:vi.fn(),confirmAction:vi.fn()}));
vi.mock("../accounts/api/accounts",()=>({listActiveAccounts:mocks.accounts}));
vi.mock("./api/transactions",()=>({createTransaction:mocks.create,listTransactions:mocks.list,listActiveCategories:mocks.categories,listEligibleRefunds:mocks.eligibleRefunds,updateTransaction:mocks.update,getTransactionAllocations:mocks.allocations,getTransactionAuditHistory:mocks.audit,voidTransaction:mocks.void,reverseTransaction:mocks.reverse,createTransfer:mocks.transfer}));
vi.mock("../../shared/confirmAction",()=>({confirmAction:mocks.confirmAction}));

describe("TransactionsPage",()=>{
 beforeEach(()=>{vi.clearAllMocks();mocks.confirmAction.mockResolvedValue(true);vi.spyOn(window,"scrollTo").mockImplementation(()=>undefined);mocks.accounts.mockResolvedValue([{id:"a",name:"Main",currency_code:"KES"},{id:"b",name:"Savings",currency_code:"KES"}]);mocks.categories.mockResolvedValue([{id:"food",name:"Food",category_type:"expense"},{id:"home",name:"Home",category_type:"expense"}]);mocks.eligibleRefunds.mockResolvedValue([{transaction_id:"original",amount_minor:1000,refundable_minor:700,occurred_at:"2026-11-01T09:00:00Z",description:"Shop"}]);mocks.list.mockResolvedValue([]);mocks.create.mockResolvedValue("tx");mocks.allocations.mockResolvedValue([]);mocks.update.mockResolvedValue(undefined);mocks.void.mockResolvedValue(undefined);mocks.reverse.mockResolvedValue("reversal");mocks.transfer.mockResolvedValue("transfer");mocks.audit.mockResolvedValue([]);});
 it("allows an uncategorized expense",async()=>{
  render(<TransactionsPage/>); await screen.findByRole("option",{name:"Main (KES)"});
  fireEvent.change(screen.getAllByLabelText("Account",{selector:"select"})[0],{target:{value:"a"}});
  fireEvent.change(screen.getByLabelText("Amount",{selector:"input"}),{target:{value:"12.50"}});
  fireEvent.click(screen.getByRole("button",{name:"Save transaction"}));
  await waitFor(()=>expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({amount_minor:1250,allocations:[]})));
 });
 it("saves a distinct refund linked to the selected expense",async()=>{
  render(<TransactionsPage/>);await screen.findByRole("option",{name:"Main (KES)"});
  fireEvent.change(screen.getByLabelText("Type"),{target:{value:"refund"}});
  fireEvent.change(screen.getAllByLabelText("Account",{selector:"select"})[0],{target:{value:"a"}});
  fireEvent.change(screen.getByLabelText("Amount",{selector:"input"}),{target:{value:"5.00"}});
  fireEvent.change(await screen.findByLabelText("Refund for expense"),{target:{value:"original"}});
  fireEvent.click(screen.getByRole("button",{name:"Save transaction"}));
  await waitFor(()=>expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({transaction_type:"refund",amount_minor:500,refund_of_transaction_id:"original",allocations:[]})));
 });
 it("shows linked refund events in the original expense audit history",async()=>{
  mocks.list.mockResolvedValue([{id:"original",account_id:"a",transaction_type:"expense",amount_minor:1000,currency_code:"KES",occurred_at:"2026-11-01T09:00:00Z",description:"Shop",status:"posted",entry_amount_minor:-1000}]);
  mocks.audit.mockResolvedValue([{action:"refund_received",previous_values:null,new_values:'{"refund_transaction_id":"refund-1","amount_minor":300}',reason:null,created_at:"2026-11-02T09:00:00Z"}]);
  render(<TransactionsPage/>);
  fireEvent.click(await screen.findByRole("button",{name:"History"}));
  expect(await screen.findByText(/refund received/)).toBeInTheDocument();
  expect(screen.getByText(/refund_transaction_id/)).toBeInTheDocument();
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
 it("loads and saves an eligible transaction edit",async()=>{
  mocks.list.mockResolvedValue([{id:"tx",account_id:"a",transaction_type:"expense",amount_minor:1200,currency_code:"KES",occurred_at:"2026-11-06T09:00:00Z",description:"Shop",status:"posted",entry_amount_minor:-1200}]);
  render(<TransactionsPage/>); fireEvent.click(await screen.findByRole("button",{name:"Edit"}));
  expect(await screen.findByRole("button",{name:"Update transaction"})).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Amount",{selector:"input"}),{target:{value:"15.00"}});
  fireEvent.click(screen.getByRole("button",{name:"Update transaction"}));
  await waitFor(()=>expect(mocks.update).toHaveBeenCalledWith("tx",expect.objectContaining({amount_minor:1500})));
 });
 it("requires a reason and confirmation before voiding",async()=>{
  mocks.list.mockResolvedValue([{id:"tx",account_id:"a",transaction_type:"expense",amount_minor:1200,currency_code:"KES",occurred_at:"2026-11-06T09:00:00Z",description:"Shop",status:"posted",entry_amount_minor:-1200}]);
  vi.spyOn(window,"prompt").mockReturnValue("Duplicate entry");
  render(<TransactionsPage/>); fireEvent.click(await screen.findByRole("button",{name:"Void"}));
  await waitFor(()=>expect(mocks.void).toHaveBeenCalledWith("tx","Duplicate entry"));
 });
 it("asks for confirmation before creating a reversal",async()=>{
  mocks.list.mockResolvedValue([{id:"tx",account_id:"a",transaction_type:"expense",amount_minor:1200,currency_code:"KES",occurred_at:"2026-11-06T09:00:00Z",description:"Shop",status:"posted",entry_amount_minor:-1200}]);
  vi.spyOn(window,"confirm").mockReturnValue(true);
  render(<TransactionsPage/>); fireEvent.click(await screen.findByRole("button",{name:"Reverse"}));
  await waitFor(()=>expect(mocks.reverse).toHaveBeenCalledWith("tx"));
 });
 it("does not reverse a transaction when the user declines",async()=>{
  mocks.list.mockResolvedValue([{id:"tx",account_id:"a",transaction_type:"expense",amount_minor:1200,currency_code:"KES",occurred_at:"2026-11-06T09:00:00Z",description:"Shop",status:"posted",entry_amount_minor:-1200}]);
  mocks.confirmAction.mockResolvedValue(false);
  render(<TransactionsPage/>); fireEvent.click(await screen.findByRole("button",{name:"Reverse"}));
  await waitFor(()=>expect(mocks.confirmAction).toHaveBeenCalled());
  expect(mocks.reverse).not.toHaveBeenCalled();
 });
 it("records an inter-account transfer and its optional fee",async()=>{
  render(<TransactionsPage/>);
  await screen.findAllByRole("option",{name:"Savings"});
  fireEvent.change(screen.getByLabelText("From account"),{target:{value:"a"}});
  fireEvent.change(screen.getByLabelText("To account"),{target:{value:"b"}});
  fireEvent.change(screen.getByLabelText("Transfer amount"),{target:{value:"20.00"}});
  fireEvent.change(screen.getByLabelText("Transfer fee"),{target:{value:"1.50"}});
  fireEvent.click(screen.getByRole("button",{name:"Save transfer"}));
  await waitFor(()=>expect(mocks.transfer).toHaveBeenCalledWith(expect.objectContaining({from_account_id:"a",to_account_id:"b",amount_minor:2000,fee_minor:150})));
 });
});
