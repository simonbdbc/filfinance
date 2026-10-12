import { Text } from "@shared/ui/Text";
import type { PropsWithChildren, ReactNode } from "react";
import { View } from "react-native";
import type { StyleProp, TextStyle, ViewStyle } from "react-native";

import { cx } from "./classNames";
import { useThemeTokens } from "../theme/theme";

type BadgeVariant = "critical" | "dark" | "info" | "muted" | "trace";

type BadgeProps = PropsWithChildren<{
  variant?: BadgeVariant;
  prefix?: ReactNode;
  className?: string;
  style?: StyleProp<ViewStyle>;
  textClassName?: string;
  textStyle?: StyleProp<TextStyle>;
}>;

const badgeClassNameByVariant: Record<BadgeVariant, string> = {
  critical: "self-start border px-2.5 py-1.5",
  dark: "self-start flex-row items-center gap-2 px-3 py-2",
  info: "self-start border px-2.5 py-1.5",
  muted: "self-start flex-row items-center gap-2 px-3 py-2",
  trace: "self-start flex-row items-center gap-2 px-3 py-2",
};

const badgeTextClassNameByVariant: Record<BadgeVariant, string> = {
  critical: "text-sm font-semibold uppercase tracking-normal",
  dark: "text-sm font-semibold tracking-normal",
  info: "text-sm font-semibold uppercase tracking-normal",
  muted: "text-sm font-semibold tracking-normal",
  trace: "text-sm font-semibold tracking-normal",
};

export function StatusBadge({
  children,
  className,
  prefix,
  style,
  textClassName,
  textStyle,
  variant = "muted",
}: BadgeProps) {
  const tokens = useThemeTokens();
  const designTheme = useThemeTokens();
  const badgeBaseStyle: ViewStyle = {
    borderRadius: designTheme.radii.control,
    maxWidth: "100%",
  };
  const badgeStyleByVariant: Record<BadgeVariant, ViewStyle> = {
    critical: {
      ...badgeBaseStyle,
      backgroundColor: tokens.colors.surface,
      borderColor: tokens.colors.dangerText,
    },
    dark: {
      ...badgeBaseStyle,
      backgroundColor: tokens.colors.primaryBackground,
    },
    info: {
      ...badgeBaseStyle,
      backgroundColor: tokens.colors.surface,
      borderColor: tokens.colors.borderSoft,
    },
    muted: {
      ...badgeBaseStyle,
      backgroundColor: designTheme.colors.background,
    },
    trace: {
      ...badgeBaseStyle,
      backgroundColor: tokens.colors.surface,
    },
  };
  const badgeTextStyleByVariant: Record<BadgeVariant, TextStyle> = {
    critical: { color: tokens.colors.dangerText },
    dark: { color: tokens.colors.primaryText },
    info: { color: tokens.colors.successText },
    muted: { color: tokens.colors.textPrimary },
    trace: { color: tokens.colors.textPrimary },
  };
  const resolvedTextStyle = badgeTextStyleByVariant[variant];
  const hasChildren =
    children !== undefined && children !== null && children !== false;

  return (
    <View
      className={cx(badgeClassNameByVariant[variant], className)}
      style={[badgeStyleByVariant[variant], style]}
    >
      {prefix}
      {hasChildren ? (
        <Text
          className={cx(badgeTextClassNameByVariant[variant], textClassName)}
          style={[{ minWidth: 0, flexShrink: 1 }, resolvedTextStyle, textStyle]}
        >
          {children}
        </Text>
      ) : null}
    </View>
  );
}
