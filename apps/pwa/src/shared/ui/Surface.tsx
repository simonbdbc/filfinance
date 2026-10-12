import type { PropsWithChildren, ReactNode } from "react";
import { View, type StyleProp, type ViewStyle } from "react-native";
import { UIPressable } from "./primitives";
import { Text, headingStyle } from "./Text";
import { useThemeTokens } from "../theme/theme";
import { cx } from "./classNames";

export function Surface({
  children,
  className,
  variant = "default",
  unframed = false,
}: PropsWithChildren<{
  unframed?: boolean;
  className?: string;
  variant?: "default" | "danger";
}>) {
  const t = useThemeTokens();
  return (
    <View
      className={cx("w-full min-w-0 max-w-full p-4 md:p-6", className)}
      style={{
        padding: unframed ? 0 : undefined,
        backgroundColor: unframed ? "transparent" : t.colors.surface,
        borderRadius: unframed ? 0 : t.radii.panel,
        borderWidth: variant === "danger" ? 1 : 0,
        borderColor: t.colors.dangerText,
      }}
    >
      {children}
    </View>
  );
}
export function SectionHeader({
  title,
  description,
  action,
  hasSeparator = true,
  onPress,
  className,
  titleClassName,
  descriptionClassName,
}: {
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  hasSeparator?: boolean;
  onPress?: () => void;
  className?: string;
  titleClassName?: string;
  descriptionClassName?: string;
}) {
  const t = useThemeTokens();
  const content = (
    <>
      <View className="min-w-0 flex-1 gap-2">
        <Text
          accessibilityRole="header"
          aria-level={2}
          className={titleClassName}
          style={headingStyle(t)}
        >
          {title}
        </Text>
        {description ? (
          typeof description === "string" || typeof description === "number" ? (
            <Text
              className={descriptionClassName}
              style={{ fontSize: 14, lineHeight: 21 }}
            >
              {description}
            </Text>
          ) : (
            description
          )
        ) : null}
      </View>
      {action}
    </>
  );
  const props = {
    className: cx(
      "min-w-0 max-w-full flex-row flex-wrap items-start gap-4 mb-4",
      className,
    ),
    style: {
      borderBottomWidth: hasSeparator ? 1 : 0,
      borderColor: t.colors.borderSoft,
      paddingBottom: hasSeparator ? 16 : 0,
    },
  };
  return onPress ? (
    <UIPressable {...props} accessibilityRole="button" onPress={onPress}>
      {content}
    </UIPressable>
  ) : (
    <View {...props}>{content}</View>
  );
}
export function DataRow({
  children,
  className,
  style,
  role,
}: PropsWithChildren<{
  className?: string;
  style?: StyleProp<ViewStyle>;
  role?: "listitem";
}>) {
  const t = useThemeTokens();
  return (
    <View
      role={role}
      className={cx("min-w-0 w-full max-w-full gap-2 border-t py-4", className)}
      style={[{ borderColor: t.colors.borderSoft }, style]}
    >
      {children}
    </View>
  );
}
