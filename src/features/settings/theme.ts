export type ThemePreference = "system" | "light" | "dark";

export function applyTheme(theme: ThemePreference) {
  const resolved = theme === "system"
    ? (typeof window.matchMedia === "function" && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
    : theme;
  document.documentElement.dataset.theme = resolved;
}
