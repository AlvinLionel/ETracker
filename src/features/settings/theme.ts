export type ThemePreference = "system" | "light" | "dark";

export function applyTheme(theme: ThemePreference) {
  const resolved = theme === "system"
    ? (typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
    : theme;
  document.documentElement.dataset.theme = resolved;
}

export function applyAppearancePreferences(theme: ThemePreference, gridOpacity: number, rowDensity: "compact" | "standard" | "comfortable") {
  applyTheme(theme);
  const dark = document.documentElement.dataset.theme === "dark";
  const rgb = dark ? "255 255 255" : "64 0 198";
  document.documentElement.style.setProperty("--workspace-grid", `rgb(${rgb} / ${gridOpacity}%)`);
  document.documentElement.dataset.previewDensity = rowDensity;
}
