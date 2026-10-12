import { useResolveClassNames } from "uniwind";
import { forwardRef } from "react";
import {
  Text as NativeText,
  type TextProps,
  type TextStyle,
  StyleSheet,
} from "react-native";
import { useThemeTokens } from "../theme/theme";

// Le texte commun garantit la police et la lisibilité, y compris dans les outils admin.
export const Text = forwardRef<NativeText, TextProps & { className?: string }>(
  function Text({ style, className, ...props }, ref) {
    const tokens = useThemeTokens();
    const classStyles = useResolveClassNames(className ?? "");
    const flat = StyleSheet.flatten([classStyles, style]) ?? {};
    return (
      <NativeText
        {...props}
        className={className}
        ref={ref}
        style={[
          {
            fontFamily: tokens.fonts.body,
            fontSize: 16,
            lineHeight: 24,
            color: tokens.colors.textPrimary,
          },
          classStyles,
          style,
          flat.fontSize && flat.fontSize < 14 ? { fontSize: 14 } : undefined,
        ]}
      />
    );
  },
);
export function headingStyle(
  tokens: ReturnType<typeof useThemeTokens>,
  size = 24,
): TextStyle {
  return {
    fontFamily: tokens.fonts.heading,
    fontSize: size,
    lineHeight: size * 1.2,
    fontWeight: "400",
    color: tokens.colors.textPrimary,
  };
}
