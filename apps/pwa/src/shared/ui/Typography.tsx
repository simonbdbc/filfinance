import { Text } from "@shared/ui/Text";
import type { PropsWithChildren } from "react";

import type { TextStyle } from "react-native";

import { cx } from "./classNames";
import { useThemeTokens } from "../theme/theme";

type TextProps = PropsWithChildren<{
  className?: string;
  style?: TextStyle;
}>;

export function EyebrowText({ children, className, style }: TextProps) {
  const tokens = useThemeTokens();
  const themedStyle: TextStyle = { color: tokens.colors.textPrimary };

  return (
    <Text
      className={cx(
        "max-w-full min-w-0 text-sm font-bold uppercase tracking-[1px]",
        className,
      )}
      style={[{ fontSize: 14, lineHeight: 21 }, themedStyle, style]}
    >
      {children}
    </Text>
  );
}

export function SubtleText({ children, className, style }: TextProps) {
  const tokens = useThemeTokens();
  const themedStyle: TextStyle = { color: tokens.colors.textPrimary };

  return (
    <Text
      className={cx(
        "max-w-full min-w-0 break-words text-sm leading-[19px]",
        className,
      )}
      style={[{ fontSize: 14, lineHeight: 21 }, themedStyle, style]}
    >
      {children}
    </Text>
  );
}

export function MetaText({ children, className, style }: TextProps) {
  const tokens = useThemeTokens();
  const themedStyle: TextStyle = { color: tokens.colors.textPrimary };

  return (
    <Text
      className={cx(
        "max-w-full min-w-0 break-words text-sm leading-[18px]",
        className,
      )}
      style={[{ fontSize: 14, lineHeight: 21 }, themedStyle, style]}
    >
      {children}
    </Text>
  );
}
