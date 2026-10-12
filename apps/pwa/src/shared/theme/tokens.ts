import { useCSSVariable } from "uniwind";
import { Platform } from "react-native";

export type ThemeName = "light" | "dark";
// Seuls les noms sémantiques sont définis ici, jamais une seconde palette.
const colorVariables = {
  background: "background",
  surface: "surface",
  border: "border",
  borderSoft: "border-soft",
  textPrimary: "foreground",
  textSecondary: "secondary",
  accentDecorative: "decorative",
  primaryBackground: "primary",
  primaryText: "primary-text",
  overlay: "overlay",
  shadow: "shadow",
  selection: "background",
  dangerText: "error",
  successText: "success",
  warningText: "warning",
} as const;
export type ThemeTokens = {
  name: ThemeName;
  colors: Record<keyof typeof colorVariables, string>;
  radii: { control: number; panel: number };
  fonts: { body: string; heading: string; mono: string };
};
const variableNames = Object.values(colorVariables).map(
  (name) => `--ff-${name}`,
);

export function useResolvedTokens(name: ThemeName): ThemeTokens {
  const values = useCSSVariable([
    ...variableNames,
    "--font-sans",
    "--font-serif",
    "--font-mono",
    "--radius-control",
    "--radius-surface",
  ]);
  const size = variableNames.length;
  const colors = Object.fromEntries(
    Object.keys(colorVariables).map((key, index) => [
      key,
      String(values[index] ?? "transparent"),
    ]),
  ) as ThemeTokens["colors"];
  return {
    name,
    colors,
    radii: {
      control: parseFloat(String(values[size + 3])) || 6,
      panel: parseFloat(String(values[size + 4])) || 8,
    },
    fonts:
      Platform.OS === "web"
        ? {
            body: String(values[size] ?? "sans-serif"),
            heading: String(values[size + 1] ?? "serif"),
            mono: String(values[size + 2] ?? "monospace"),
          }
        : {
            body: Platform.OS === "ios" ? "Arial" : "sans-serif",
            heading: Platform.OS === "ios" ? "Georgia" : "serif",
            mono: Platform.OS === "ios" ? "Menlo" : "monospace",
          },
  };
}
