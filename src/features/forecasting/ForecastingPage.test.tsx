import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ForecastingPage from "./ForecastingPage";

const invoke=vi.fn();
vi.mock("@tauri-apps/api/core",()=>({invoke:(...args:unknown[])=>invoke(...args)}));

const assumptions={history_start:"2026-04-01",history_end:"2026-09-30",active_months_with_activity:5,average_income_minor:100000,average_non_subscription_expense_minor:40000,normalized_active_subscription_minor:10000,average_savings_contribution_minor:5000,starting_balance_minor:300000};
const forecast={id:"forecast-1",generated_at:"2026-11-07T10:00:00Z",currency_code:"KES",period_months:1,confidence:"moderate",assumptions,limitations:["Seasonality is not modeled."],periods:[{month_start:"2020-01-01",predicted_income_minor:100000,predicted_expense_minor:50000,predicted_subscription_minor:10000,predicted_savings_minor:5000,predicted_ending_balance_minor:350000,planned_income_minor:null,planned_expense_minor:50000,planned_savings_minor:5000,evaluation:null}]};

describe("ForecastingPage",()=>{
  beforeEach(()=>{invoke.mockReset();invoke.mockImplementation((command:string,args?:{categoryType?:string})=>command==="list_forecasts"?Promise.resolve([forecast]):command==="get_forecast_schedule"?Promise.resolve({enabled:false,currency_code:"KES",period_months:3}):command==="set_forecast_schedule"?Promise.resolve(true):command==="list_active_accounts"?Promise.resolve([{id:"cash",name:"Cash",account_type:"cash",currency_code:"KES",institution_name:null,notes:null,created_at:"",updated_at:"",archived_at:null}]):command==="list_active_categories"?Promise.resolve(args?.categoryType==="income"?[{id:"salary",name:"Salary",category_type:"income"}]:[{id:"food",name:"Food",category_type:"expense"}]):Promise.resolve("new-snapshot"));});
  it("generates snapshots from user choices and evaluates completed periods",async()=>{
    render(<ForecastingPage/>);
    expect(await screen.findByText("KES · 1-month forecast")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Deterministic ending balance and monthly cash flow forecast" })).toBeInTheDocument();
    expect(screen.getByRole("table", { name: /Exact deterministic forecast values/ })).toHaveTextContent("Jan 20");
    expect(screen.getAllByText(/history coverage: moderate/i)).toHaveLength(2);
    expect(screen.getByText(/Seasonality is not modeled/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "6 mo" }));
    fireEvent.click(screen.getAllByRole("button",{name:"Generate forecast"})[0]);
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("generate_forecast",{request:{currency_code:"KES",period_months:6}}));
    const evaluateButton=await screen.findByRole("button",{name:"Evaluate"});
    fireEvent.click(evaluateButton);
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("evaluate_forecast",{snapshotId:"forecast-1",monthStart:"2020-01-01"}));
  });
  it("saves opt-in monthly forecast generation settings",async()=>{
    render(<ForecastingPage/>);
    fireEvent.click(screen.getByRole("checkbox",{name:/Generate monthly/}));
    fireEvent.click(screen.getByRole("button", { name: "6 mo" }));
    fireEvent.click(screen.getByRole("button",{name:"Save schedule"}));
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("set_forecast_schedule",{schedule:{enabled:true,currency_code:"KES",period_months:6}}));
  });
  it("promotes a forecast to a categorized budget template without posting transactions",async()=>{
    render(<ForecastingPage/>);
    await waitFor(()=>expect(screen.getByLabelText("Forecast income category")).toHaveValue("salary"));
    await waitFor(()=>expect(screen.getByLabelText("Forecast expense category")).toHaveValue("food"));
    fireEvent.click(await screen.findByRole("button",{name:"Promote to budget plan"}));
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("create_budget_template",{request:{name:"Forecast 2026-11-07",currency_code:"KES",items:[{item_type:"income",category_id:"salary",label:"Forecast monthly income",amount_minor:100000},{item_type:"expense",category_id:"food",label:"Forecast monthly expenses",amount_minor:50000},{item_type:"savings",category_id:null,label:"Forecast monthly savings",amount_minor:5000}]}}));
    expect(await screen.findByRole("status")).toHaveTextContent(/does not change transactions or existing monthly budgets/);
    expect(invoke).not.toHaveBeenCalledWith("create_transaction",expect.anything());
  });
  it("exports the current scenario assumptions and projected periods",async()=>{
    const create=vi.spyOn(URL,"createObjectURL").mockReturnValue("blob:scenario");
    const revoke=vi.spyOn(URL,"revokeObjectURL").mockImplementation(()=>{});
    render(<ForecastingPage/>);
    fireEvent.click(await screen.findByRole("button",{name:"Export simulation audit"}));
    expect(create).toHaveBeenCalledWith(expect.any(Blob));
    expect(revoke).toHaveBeenCalledWith("blob:scenario");
  });
});
