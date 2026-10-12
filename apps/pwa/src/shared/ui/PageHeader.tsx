import { createContext, useContext, type ReactNode } from "react";
export const PageHeaderInLayout = createContext(false);
import { View, useWindowDimensions } from "react-native";
import { Text, headingStyle } from "./Text";
import { useThemeTokens } from "../theme/theme";
export function PageHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  const t = useThemeTokens();
  const inLayout = useContext(PageHeaderInLayout);
  const { width } = useWindowDimensions();
  if (inLayout)
    return description || action ? (
      <View className="flex-row flex-wrap items-center justify-between gap-4">
        {description ? (
          <Text style={{ fontSize: 14, flex: 1, minWidth: 0 }}>
            {description}
          </Text>
        ) : null}
        {action}
      </View>
    ) : null;
  return (
    <>
      <View
        className="min-w-0 w-full max-w-full items-start justify-between gap-4 mb-2"
        style={{ flexDirection: width < 768 ? "column" : "row" }}
      >
        <View
          className="min-w-0 gap-2"
          style={{ flex: width < 768 ? undefined : 1 }}
        >
          <Text
            accessibilityRole="header"
            aria-level={1}
            style={headingStyle(t, width >= 1024 ? 36 : 30)}
          >
            {title}
          </Text>
          {description ? (
            <Text style={{ fontSize: 14, lineHeight: 21 }}>{description}</Text>
          ) : null}
        </View>
        {action}
      </View>
    </>
  );
}
