import type { PropsWithChildren } from "react";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { Platform } from "react-native";
import AsyncStorage from "@react-native-async-storage/async-storage";
import { Uniwind, useUniwind } from "uniwind";
import { OverlayProvider } from "@gluestack-ui/core/overlay/creator";
import { type ThemeName, type ThemeTokens, useResolvedTokens } from "./tokens";
import { readWebTheme, isThemeName, themeStorageKey } from "./preference";

export type { ThemeName, ThemeTokens } from "./tokens";
type ThemeContextValue = {
  themeName: ThemeName;
  tokens: ThemeTokens;
  setThemeName: (name: ThemeName) => void;
  toggleTheme: () => void;
};
const ThemeContext = createContext<ThemeContextValue | null>(null);
// L'amorçage Web précède le montage React ; aucun choix système implicite.
const initialTheme = readWebTheme();
Uniwind.setTheme(initialTheme);

export function ThemeProvider({ children }: PropsWithChildren) {
  const { theme } = useUniwind();
  const themeName: ThemeName = isThemeName(theme) ? theme : "light";
  const [ready, setReady] = useState(Platform.OS === "web");
  const tokens = useResolvedTokens(themeName);
  const setThemeName = useCallback((name: ThemeName) => {
    if (!isThemeName(name)) return;
    Uniwind.setTheme(name);
    // L'échec d'écriture ne doit jamais annuler le choix pendant la visite.
    void AsyncStorage.setItem(themeStorageKey, name).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (Platform.OS === "web") return;
    let cancelled = false;
    void AsyncStorage.getItem(themeStorageKey)
      .then((stored) => {
        if (!cancelled && isThemeName(stored)) {
          Uniwind.setTheme(stored);
        }
      })
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);
  const value = useMemo(
    () => ({
      themeName,
      tokens,
      setThemeName,
      toggleTheme: () => setThemeName(themeName === "light" ? "dark" : "light"),
    }),
    [themeName, tokens, setThemeName],
  );

  return (
    <ThemeContext.Provider value={value}>
      {ready ? <OverlayProvider>{children}</OverlayProvider> : null}
    </ThemeContext.Provider>
  );
}
export function useTheme() {
  const value = useContext(ThemeContext);
  if (!value) throw new Error("ThemeProvider manquant");
  return value;
}
export function useThemeTokens() {
  return useTheme().tokens;
}
