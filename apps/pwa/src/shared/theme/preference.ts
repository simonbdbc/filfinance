export const themeStorageKey = "filfinance.theme";
export function isThemeName(value: unknown): value is "light" | "dark" {
  return value === "light" || value === "dark";
}
export function readWebTheme(): "light" | "dark" {
  try {
    const stored =
      typeof window === "undefined"
        ? null
        : window.localStorage.getItem(themeStorageKey);
    return isThemeName(stored) ? stored : "light";
  } catch {
    return "light";
  }
}
