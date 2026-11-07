import AccountsPage from "./features/accounts/AccountsPage";
import TransactionsPage from "./features/transactions/TransactionsPage";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ImportsPage from "./features/imports/ImportsPage";
import BudgetingPage from "./features/planning/BudgetingPage";
import SavingsPage from "./features/planning/SavingsPage";
import SubscriptionsPage from "./features/planning/SubscriptionsPage";
import AnalyticsPage from "./features/analytics/AnalyticsPage";
import ForecastingPage from "./features/forecasting/ForecastingPage";
import DashboardPage from "./features/dashboard/DashboardPage";
import SettingsPage from "./features/settings/SettingsPage";
import { applyTheme, type ThemePreference } from "./features/settings/theme";

const navigationItems = [
  { label: "Dashboard", section: "Overview" },
  { label: "Accounts", section: "Finance" },
  { label: "Transactions", section: "Finance" },
  { label: "Imports", section: "Finance" },
  { label: "Budgeting", section: "Planning" },
  { label: "Savings", section: "Planning" },
  { label: "Subscriptions", section: "Planning" },
  { label: "Analytics", section: "Insights" },
  { label: "Forecasting", section: "Insights" },
  { label: "Settings", section: "System" },
];

function App() {
  const [activePage, setActivePage] = useState("Dashboard");
  const availablePages = ["Dashboard", "Accounts", "Transactions", "Imports", "Budgeting", "Savings", "Subscriptions", "Analytics", "Forecasting", "Settings"];

  useEffect(() => {
    let active = true;
    let theme: ThemePreference = "system";
    applyTheme(theme);
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const updateSystemTheme = () => { if (theme === "system") applyTheme(theme); };
    media.addEventListener("change", updateSystemTheme);
    invoke<{ theme: ThemePreference }>("get_user_preferences")
      .then((preferences) => { if (active) { theme = preferences.theme; applyTheme(theme); } })
      .catch(() => {});
    return () => { active = false; media.removeEventListener("change", updateSystemTheme); };
  }, []);

  return (
    <div className="app-shell flex min-h-screen">
      {/* Sidebar */}
      <aside className="app-sidebar hidden w-64 shrink-0 flex-col border-r lg:flex">
        <div className="flex h-16 items-center gap-3 border-b border-slate-200 px-6">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-emerald-700 text-lg font-bold text-white">
            E
          </div>
          <div>
            <h1 className="text-base font-bold tracking-tight">ETracker</h1>
            <p className="text-xs text-slate-500">Personal finance</p>
          </div>
        </div>

        <nav aria-label="Primary navigation" className="flex-1 space-y-7 overflow-y-auto px-4 py-6">
          {["Overview", "Finance", "Planning", "Insights", "System"].map(
            (section) => {
              const items = navigationItems.filter(
                (item) => item.section === section,
              );

              return (
                <div key={section}>
                  <p className="mb-2 px-3 text-[10px] font-semibold uppercase tracking-[0.14em] text-slate-400">
                    {section}
                  </p>

                  <div className="space-y-1">
                    {items.map((item) => {
                      const isActive = item.label === activePage;

                      return (
                        <button key={item.label} type="button" onClick={() => { if (availablePages.includes(item.label)) setActivePage(item.label); }} aria-current={isActive ? "page" : undefined}
                          className={`flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium transition-colors ${isActive ? "app-nav-active" : "app-nav-idle"}`}
                        >
                          <span className={`h-1.5 w-1.5 rounded-full ${isActive ? "bg-emerald-600" : "bg-slate-300"}`}/>
                          {item.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            },
          )}
        </nav>

        <div className="border-t border-slate-200 px-5 py-4">
          <p className="text-xs font-medium text-slate-700">Local workspace</p>
          <p className="mt-1 text-xs text-slate-500">Your financial data stays on this device.</p>
        </div>
      </aside>

      {/* Main workspace */}
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="app-header flex min-h-16 flex-wrap items-center justify-between gap-3 border-b px-5 py-2 sm:px-8">
          <div>
            <p className="text-xs text-slate-500">Finance / {activePage}</p>
            <h2 className="text-sm font-semibold text-slate-900">{activePage === "Accounts" ? "Account management" : ["Budgeting","Savings","Subscriptions"].includes(activePage) ? "Financial planning" : activePage === "Transactions" ? "Transaction management" : activePage === "Imports" ? "Statement imports" : activePage}</h2>
          </div>

          <div className="flex items-center gap-2 rounded-full border border-slate-200 px-3 py-1.5">
            <span className="h-2 w-2 rounded-full bg-emerald-500" />
            <span className="text-xs font-medium text-slate-600">Local mode</span>
          </div>
        </header>

        <nav aria-label="Compact navigation" className="app-compact-nav overflow-x-auto border-b px-3 py-2 lg:hidden">
          <div className="flex min-w-max gap-1">
            {navigationItems.map((item) => {
              const isActive = item.label === activePage;
              return (
                <button
                  key={item.label}
                  type="button"
                  onClick={() => setActivePage(item.label)}
                  aria-current={isActive ? "page" : undefined}
                  className={`rounded-lg px-3 py-2 text-sm font-medium transition-colors ${isActive ? "app-nav-active" : "app-nav-idle"}`}
                >
                  {item.label}
                </button>
              );
            })}
          </div>
        </nav>

        <main className="flex-1 px-5 py-8 sm:px-8 lg:px-10">
          {activePage === "Dashboard" ? <DashboardPage navigate={setActivePage} /> : activePage === "Accounts" ? <AccountsPage /> : activePage === "Transactions" ? <TransactionsPage /> : activePage === "Imports" ? <ImportsPage /> : activePage === "Budgeting" ? <BudgetingPage /> : activePage === "Savings" ? <SavingsPage /> : activePage === "Subscriptions" ? <SubscriptionsPage /> : activePage === "Analytics" ? <AnalyticsPage /> : activePage === "Forecasting" ? <ForecastingPage /> : activePage === "Settings" ? <SettingsPage /> : <DashboardPage navigate={setActivePage} />}
        </main>
      </div>
    </div>
  );
}

export default App;
