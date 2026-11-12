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
import CategoriesPage from "./features/transactions/CategoriesPage";
import { applyAppearancePreferences, type ThemePreference } from "./features/settings/theme";
import { Activity, CalendarDays, ChartNoAxesCombined, Download, Landmark, LayoutDashboard, Moon, Monitor, PanelLeftClose, PiggyBank, Repeat2, Search, Settings2, Sun, Tags, TrendingUp, WalletCards } from "lucide-react";

const navigationItems = [
  { label: "Dashboard", section: "Overview", icon: LayoutDashboard },
  { label: "Accounts", section: "Finance", icon: Landmark },
  { label: "Imports", section: "Finance", icon: Download },
  { label: "Categories", section: "Finance", icon: Tags },
  { label: "Transactions", section: "Finance", icon: WalletCards },
  { label: "Budgeting", section: "Planning", icon: Activity },
  { label: "Savings", section: "Planning", icon: PiggyBank },
  { label: "Subscriptions", section: "Planning", icon: Repeat2 },
  { label: "Analytics", section: "Insights", icon: ChartNoAxesCombined },
  { label: "Forecasting", section: "Insights", icon: TrendingUp },
  { label: "Settings", section: "System", icon: Settings2 },
];

function App() {
  const [activePage, setActivePage] = useState("Dashboard");
  const [themePreference, setThemePreference] = useState<ThemePreference>("system");
  const [appearance, setAppearance] = useState<{ grid_opacity: number; row_density: "compact" | "standard" | "comfortable" }>({ grid_opacity: 5, row_density: "standard" });
  const [themeError, setThemeError] = useState("");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [planningMonth, setPlanningMonth] = useState(() => {
    const date = new Date();
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
  });
  const [searchQuery, setSearchQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [ledgerResults, setLedgerResults] = useState<{kind:string;id:string;title:string;detail:string;page:string}[]>([]);
  const [ledgerSearchBusy, setLedgerSearchBusy] = useState(false);
  const [ledgerSearchError, setLedgerSearchError] = useState("");
  const availablePages = ["Dashboard", "Accounts", "Transactions", "Imports", "Categories", "Budgeting", "Savings", "Subscriptions", "Analytics", "Forecasting", "Settings"];
  const searchResults = navigationItems.filter((item) => `${item.label} ${item.section}`.toLowerCase().includes(searchQuery.trim().toLowerCase()));
  const now = new Date();
  const planningMonthLabel = new Intl.DateTimeFormat("en", { month: "long", year: "numeric" }).format(new Date(`${planningMonth}-01T00:00:00`));
  const calendarQuarter = `Calendar Q${Math.floor(now.getMonth() / 3) + 1} · ${now.getFullYear()}`;

  useEffect(() => {
    const query = searchQuery.trim();
    setLedgerSearchError("");
    if (query.length < 2) { setLedgerResults([]); setLedgerSearchBusy(false); return; }
    let active = true;
    const timer = window.setTimeout(() => {
      setLedgerSearchBusy(true);
      invoke<typeof ledgerResults>("search_ledger", { query, limit: 12 })
        .then((results) => { if (active) setLedgerResults(results); })
        .catch(() => { if (active) setLedgerSearchError("Ledger search is unavailable."); })
        .finally(() => { if (active) setLedgerSearchBusy(false); });
    }, 180);
    return () => { active = false; window.clearTimeout(timer); };
  }, [searchQuery]);

  useEffect(() => {
    let active = true;
    invoke<{ theme: ThemePreference; grid_opacity?: number; row_density?: "compact" | "standard" | "comfortable" }>("get_user_preferences")
      .then((preferences) => { if (active) { setThemePreference(preferences.theme); setAppearance({ grid_opacity: preferences.grid_opacity ?? 5, row_density: preferences.row_density ?? "standard" }); } })
      .catch(() => {});
    return () => { active = false; };
  }, []);

  useEffect(() => {
    applyAppearancePreferences(themePreference, appearance.grid_opacity, appearance.row_density);
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const updateSystemTheme = () => { if (themePreference === "system") applyAppearancePreferences("system", appearance.grid_opacity, appearance.row_density); };
    media.addEventListener("change", updateSystemTheme);
    return () => media.removeEventListener("change", updateSystemTheme);
  }, [themePreference, appearance]);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        document.getElementById("global-command-search")?.focus();
      }
    };
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  async function changeTheme(theme: ThemePreference) {
    setThemeError("");
    try {
      const preferences = await invoke<{ subscription_reminders_enabled: boolean; subscription_reminder_days: number; theme: ThemePreference; grid_opacity: number; row_density: "compact" | "standard" | "comfortable" }>("get_user_preferences");
      await invoke("set_user_preferences", { preferences: { ...preferences, theme } });
      setThemePreference(theme);
      setAppearance({ grid_opacity: preferences.grid_opacity, row_density: preferences.row_density });
      applyAppearancePreferences(theme, preferences.grid_opacity, preferences.row_density);
    } catch {
      setThemeError("Theme preference could not be saved.");
    }
  }

  return (
    <div className="app-shell flex min-h-screen flex-col">
      <header className="app-header app-topbar z-30 flex h-12 shrink-0 items-center gap-4 border-b px-4 sm:px-5">
        <div className="flex min-w-0 items-center gap-4 lg:w-[430px]">
          <span className="font-mono text-[11px] font-semibold uppercase tracking-[0.08em] text-slate-600">ETracker Desktop</span>
          <span className="hidden h-5 w-px bg-slate-200 sm:block" />
          <span className="hidden items-center gap-2 rounded-sm bg-violet-50 px-2 py-1 font-mono text-xs font-medium text-slate-800 sm:inline-flex"><span className="h-2 w-2 rounded-full bg-emerald-700" />Tauri · KES</span>
        </div>
        <span title="ETracker currently uses one local ledger on this device." className="hidden min-w-0 items-center border-l border-slate-200 pl-4 text-sm text-slate-700 xl:flex">Primary Personal Ledger</span>
        <div className="command-search relative mx-auto hidden max-w-[360px] flex-1 md:block">
          <label className="flex h-8 items-center gap-2 rounded-sm border border-slate-200 bg-violet-50/60 px-2.5"><Search size={16} className="shrink-0 text-slate-500"/><input id="global-command-search" aria-label="Search modules" value={searchQuery} onFocus={() => setSearchOpen(true)} onChange={(event) => { setSearchQuery(event.target.value); setSearchOpen(true); }} onKeyDown={(event) => { if (event.key === "Escape") { setSearchOpen(false); setSearchQuery(""); } if (event.key === "Enter" && searchResults[0]) { setActivePage(searchResults[0].label); setSearchOpen(false); setSearchQuery(""); } }} placeholder="Search ledger, accounts, rules…" className="min-w-0 flex-1 border-0 bg-transparent p-0 text-xs outline-none"/><kbd className="rounded border border-slate-200 bg-white px-1.5 py-0.5 font-mono text-[10px]">Ctrl+K</kbd></label>
          {searchOpen && <div className="absolute left-0 right-0 top-full z-50 mt-1 max-h-[min(70vh,28rem)] overflow-y-auto rounded-md border border-slate-200 bg-white p-1 shadow-lg">{searchQuery.trim().length < 2 ? <><p className="px-2 py-2 text-[10px] font-semibold uppercase tracking-wide text-slate-400">Go to</p>{searchResults.slice(0,6).map((item)=><button key={item.label} type="button" onClick={()=>{setActivePage(item.label);setSearchOpen(false);setSearchQuery("");}} className="flex w-full items-center gap-2 rounded-sm px-2 py-2 text-left text-xs hover:bg-violet-50"><item.icon size={14}/><span className="flex-1">{item.label}</span><span className="text-slate-400">{item.section}</span></button>)}</> : <>{ledgerSearchBusy && <p className="px-2 py-2 text-xs text-slate-500">Searching your ledger…</p>}{ledgerSearchError && <p role="alert" className="px-2 py-2 text-xs text-rose-700">{ledgerSearchError}</p>}{!ledgerSearchBusy && !ledgerSearchError && ledgerResults.length===0 && <p className="px-2 py-3 text-xs text-slate-500">No matching ledger records.</p>}{ledgerResults.map((item)=><button key={`${item.kind}-${item.id}`} type="button" onClick={()=>{setActivePage(item.page);setSearchOpen(false);setSearchQuery("");}} className="block w-full rounded-sm px-2 py-2 text-left hover:bg-violet-50"><span className="block truncate text-xs font-semibold">{item.title}</span><span className="mt-0.5 block truncate text-[10px] text-slate-500">{item.kind} · {item.detail}</span></button>)}</>}<p className="border-t border-slate-100 px-2 py-1.5 text-[10px] text-slate-400">Search covers posted transactions, accounts, categories and budget templates.</p></div>}
        </div>
        <span className="ml-auto hidden items-center gap-2 border-l border-slate-200 pl-4 font-mono text-xs text-slate-600 sm:flex"><CalendarDays size={14}/>{calendarQuarter}</span>
      </header>
      <div className="flex min-h-0 flex-1">
      {/* Sidebar */}
      <aside className={`app-sidebar sticky top-12 hidden h-[calc(100vh-3rem)] shrink-0 flex-col border-r transition-[width] lg:flex ${sidebarCollapsed ? "w-[76px]" : "w-60"}`}>
        <div className={`flex h-[68px] shrink-0 items-center border-b border-slate-200 ${sidebarCollapsed ? "justify-center px-2" : "gap-3 px-4"}`}>
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-violet-700 text-lg font-bold text-white">E</div>
          {!sidebarCollapsed && <><div className="min-w-0 flex-1"><h1 className="text-base font-bold tracking-tight">ETracker</h1><p className="font-mono text-[10px] font-semibold uppercase tracking-[0.12em] text-violet-700">Precision Tracker</p></div><button type="button" title="Collapse navigation" aria-label="Collapse navigation" onClick={() => setSidebarCollapsed(true)} className="rounded-sm p-1.5 text-slate-600"><PanelLeftClose size={18}/></button></>}
          {sidebarCollapsed && <button type="button" title="Expand navigation" aria-label="Expand navigation" onClick={() => setSidebarCollapsed(false)} className="rounded-sm p-1.5 text-slate-600"><PanelLeftClose size={18}/></button>}
        </div>

        <nav aria-label="Primary navigation" className={`min-h-0 flex-1 space-y-6 overflow-y-auto overscroll-contain py-5 ${sidebarCollapsed ? "px-2" : "px-3"}`}>
          {["Overview", "Finance", "Planning", "Insights", "System"].map(
            (section) => {
              const items = navigationItems.filter(
                (item) => item.section === section,
              );

              return (
                <div key={section}>
                  <p className={`mb-2 px-3 font-mono text-[10px] font-semibold uppercase tracking-[0.1em] text-slate-400 ${sidebarCollapsed ? "sr-only" : ""}`}>
                    {section}
                  </p>

                  <div className="space-y-1">
                    {items.map((item) => {
                      const isActive = item.label === activePage;

                      return (
                        <button key={item.label} type="button" onClick={() => { if (availablePages.includes(item.label)) setActivePage(item.label); }} aria-current={isActive ? "page" : undefined}
                          title={sidebarCollapsed ? item.label : undefined}
                          className={`flex min-h-10 w-full items-center gap-3 rounded-md border-l-2 py-2 text-left text-sm font-medium transition-colors ${sidebarCollapsed ? "justify-center px-2" : "px-3"} ${isActive ? "app-nav-active" : "app-nav-idle"}`}
                        >
                          <item.icon aria-hidden="true" size={17} strokeWidth={1.8} />
                          {!sidebarCollapsed && item.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            },
          )}
        </nav>

        <div className={`app-theme-control border-t border-slate-200 py-3 ${sidebarCollapsed ? "px-2" : "px-3"}`}>
          {!sidebarCollapsed && <p className="mb-2 px-1 font-mono text-[10px] font-semibold uppercase tracking-[0.12em] text-slate-400">Theme</p>}
          <div className="grid grid-cols-3 gap-1 rounded-md bg-slate-100 p-1" role="group" aria-label="Theme preference">
            {(["light", "dark", "system"] as const).map((theme) => {
              const ThemeIcon = theme === "light" ? Sun : theme === "dark" ? Moon : Monitor;
              return <button key={theme} type="button" aria-label={`${theme} theme`} title={`${theme} theme`} aria-pressed={themePreference === theme} onClick={() => void changeTheme(theme)} className={`flex h-8 items-center justify-center rounded-sm ${themePreference === theme ? "bg-violet-700 text-white" : "app-nav-idle"}`}>
                <ThemeIcon aria-hidden="true" size={15} />
              </button>
            })}
          </div>
          {themeError && <p role="alert" className="mt-2 text-xs text-rose-700">{themeError}</p>}
          {!sidebarCollapsed && <div className="mt-3 flex items-center gap-3 rounded-md border border-slate-200 bg-white px-3 py-2"><div className="flex h-9 w-9 items-center justify-center rounded-md bg-emerald-50 text-emerald-800"><WalletCards size={18} aria-hidden="true" /></div><div className="min-w-0"><p className="truncate text-sm font-semibold">Local ledger</p><p className="text-xs text-slate-500">Private workspace</p></div></div>}
        </div>

      </aside>

      {/* Main workspace */}
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="app-context-bar flex min-h-14 shrink-0 items-center justify-between gap-3 border-b px-4 sm:px-5">
          <div className="flex min-w-0 items-center gap-3"><span className="text-sm text-slate-600">Ledger</span><span className="text-slate-300">/</span><div className="min-w-0"><p className="truncate text-[10px] text-slate-500">{activePage} / Workspace</p><h2 className="truncate text-sm font-semibold">{activePage === "Accounts" ? "Account management" : ["Budgeting", "Savings", "Subscriptions"].includes(activePage) ? "Financial planning" : activePage === "Transactions" ? "Transaction management" : activePage === "Imports" ? "Statement imports" : activePage}</h2></div><span className="hidden h-5 w-px bg-slate-200 sm:block"/><label className="hidden items-center gap-2 rounded-sm border border-slate-200 bg-violet-50/60 px-2.5 py-1.5 font-mono text-xs text-slate-700 sm:flex"><CalendarDays size={14}/><span>{planningMonthLabel}</span><input aria-label="Planning month" type="month" value={planningMonth} onChange={(event) => setPlanningMonth(event.target.value)} className="w-28 border-0 bg-transparent p-0 text-xs outline-none"/></label></div>
          <div className="flex shrink-0 items-center gap-2"><button type="button" title="Open statement import reconciliation" onClick={() => setActivePage("Imports")} className="hidden items-center gap-2 rounded-sm border border-slate-200 bg-white px-3 py-2 text-xs font-semibold sm:inline-flex">Reconcile statement</button><button type="button" onClick={() => setActivePage("Transactions")} className="rounded-sm bg-violet-700 px-3 py-2 text-xs font-semibold text-white">+ Add transaction</button></div>
        </div>

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
                  className={`rounded-md px-3 py-2 text-sm font-medium transition-colors ${isActive ? "app-nav-active" : "app-nav-idle"}`}
                >
                  {item.label}
                </button>
              );
            })}
          </div>
        </nav>

        <main className={`flex-1 px-3 py-4 sm:px-5 lg:px-6 ${activePage === "Dashboard" ? "app-dashboard-main" : ""}`}>
          {activePage === "Dashboard" ? <DashboardPage navigate={setActivePage} /> : activePage === "Accounts" ? <AccountsPage /> : activePage === "Transactions" ? <TransactionsPage /> : activePage === "Imports" ? <ImportsPage navigate={setActivePage} /> : activePage === "Categories" ? <CategoriesPage /> : activePage === "Budgeting" ? <BudgetingPage initialMonth={planningMonth} navigate={setActivePage} /> : activePage === "Savings" ? <SavingsPage /> : activePage === "Subscriptions" ? <SubscriptionsPage /> : activePage === "Analytics" ? <AnalyticsPage /> : activePage === "Forecasting" ? <ForecastingPage navigate={setActivePage} /> : activePage === "Settings" ? <SettingsPage /> : <DashboardPage navigate={setActivePage} />}
        </main>
      </div>
      </div>
    </div>
  );
}

export default App;
