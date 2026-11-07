import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ForecastingPage from "./ForecastingPage";

const invoke=vi.fn();
vi.mock("@tauri-apps/api/core",()=>({invoke:(...args:unknown[])=>invoke(...args)}));

const assumptions={history_start:"2026-04-01",history_end:"2026-09-30",active_months_with_activity:5,average_income_minor:100000,average_non_subscription_expense_minor:40000,normalized_active_subscription_minor:10000,average_savings_contribution_minor:5000,starting_balance_minor:300000};
const forecast={id:"forecast-1",generated_at:"2026-11-07T10:00:00Z",currency_code:"KES",period_months:1,confidence:"moderate",assumptions,limitations:["Seasonality is not modeled."],periods:[{month_start:"2020-01-01",predicted_income_minor:100000,predicted_expense_minor:50000,predicted_subscription_minor:10000,predicted_savings_minor:5000,predicted_ending_balance_minor:350000,planned_income_minor:null,planned_expense_minor:50000,planned_savings_minor:5000,evaluation:null}]};

describe("ForecastingPage",()=>{
  beforeEach(()=>{invoke.mockReset();invoke.mockImplementation((command:string)=>command==="list_forecasts"?Promise.resolve([forecast]):command==="get_forecast_schedule"?Promise.resolve({enabled:false,currency_code:"KES",period_months:3}):command==="set_forecast_schedule"?Promise.resolve(true):command==="list_active_accounts"?Promise.resolve([{id:"cash",name:"Cash",account_type:"cash",currency_code:"KES",institution_name:null,notes:null,created_at:"",updated_at:"",archived_at:null}]):Promise.resolve("new-snapshot"));});
  it("generates snapshots from user choices and evaluates completed periods",async()=>{
    render(<ForecastingPage/>);
    expect(await screen.findByText("KES · 1-month forecast")).toBeInTheDocument();
    expect(screen.getByText(/history coverage: moderate/)).toBeInTheDocument();
    expect(screen.getByText(/Seasonality is not modeled/)).toBeInTheDocument();
    fireEvent.change(screen.getAllByLabelText("Forecast length")[0],{target:{value:"6"}});
    fireEvent.click(screen.getAllByRole("button",{name:"Generate forecast"})[0]);
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("generate_forecast",{request:{currency_code:"KES",period_months:6}}));
    const evaluateButton=await screen.findByRole("button",{name:"Evaluate"});
    fireEvent.click(evaluateButton);
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("evaluate_forecast",{snapshotId:"forecast-1",monthStart:"2020-01-01"}));
  });
  it("saves opt-in monthly forecast generation settings",async()=>{
    render(<ForecastingPage/>);
    fireEvent.click(screen.getByRole("checkbox",{name:/Generate monthly/}));
    fireEvent.change(screen.getAllByLabelText("Forecast length")[0],{target:{value:"6"}});
    fireEvent.click(screen.getByRole("button",{name:"Save schedule"}));
    await waitFor(()=>expect(invoke).toHaveBeenCalledWith("set_forecast_schedule",{schedule:{enabled:true,currency_code:"KES",period_months:6}}));
  });
});
