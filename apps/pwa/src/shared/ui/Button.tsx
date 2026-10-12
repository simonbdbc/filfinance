import type { PropsWithChildren, ReactNode } from "react";
import { IconColorContext } from "./Icon";
import { UIButton } from "./primitives";
import { useThemeTokens } from "../theme/theme";
import { cx } from "./classNames";

type ButtonProps = PropsWithChildren<{
  variant?: "primary" | "secondary";
  disabled?: boolean;
  className?: string;
  prefix?: ReactNode;
  suffix?: ReactNode;
  textClassName?: string;
  onPress: () => void;
  accessibilityLabel?: string;
  selected?: boolean;
}>;
export function Button({
  children,
  variant = "secondary",
  className,
  textClassName,
  disabled,
  prefix,
  suffix,
  onPress,
  accessibilityLabel,
  selected,
}: ButtonProps) {
  const tokens = useThemeTokens();
  const primary = (variant === "primary" || selected) && !disabled;
  return (
    <UIButton
      onPress={onPress}
      isDisabled={disabled}
      accessibilityLabel={accessibilityLabel}
      accessibilityState={{ disabled: Boolean(disabled), selected }}
      aria-pressed={selected}
      className={cx(
        "min-w-0 max-w-full flex-row items-center justify-center gap-2",
        className,
      )}
      style={{
        opacity: disabled ? 0.6 : 1,
        minHeight: 44,
        minWidth: 44,
        paddingHorizontal: children ? 16 : 10,
        paddingVertical: 10,
        borderWidth: 1,
        borderRadius: tokens.radii.control,
        borderColor: primary
          ? tokens.colors.primaryBackground
          : tokens.colors.border,
        backgroundColor: primary
          ? tokens.colors.primaryBackground
          : tokens.colors.surface,
      }}
    >
      <IconColorContext.Provider
        value={primary ? tokens.colors.primaryText : tokens.colors.textPrimary}
      >
        {prefix}
        {children ? (
          <UIButton.Text
            className={cx("min-w-0 shrink text-center", textClassName)}
            style={{
              fontFamily: tokens.fonts.body,
              fontSize: 16,
              lineHeight: 22,
              fontWeight: "600",
              color: primary
                ? tokens.colors.primaryText
                : tokens.colors.textPrimary,
            }}
          >
            {children}
          </UIButton.Text>
        ) : null}
        {suffix}
      </IconColorContext.Provider>
    </UIButton>
  );
}
