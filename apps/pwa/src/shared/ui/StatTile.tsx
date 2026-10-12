import { Text } from "@shared/ui/Text";
import type { ReactNode } from "react";
import { View } from "react-native";

import { cx } from "./classNames";
import { EyebrowText } from "./Typography";
import { useThemeTokens } from "../theme/theme";

type StatTileProps = {
  label: ReactNode;
  value: ReactNode;
  className?: string;
};

export function StatTile({ className, label, value }: StatTileProps) {
  const tokens = useThemeTokens();
  const designTheme = useThemeTokens();

  return (
    <View
      className={cx("min-w-[88px] flex-1 gap-2 border px-3 py-3", className)}
      style={{
        backgroundColor: designTheme.colors.surface,
        borderColor: designTheme.colors.borderSoft,
        borderRadius: designTheme.radii.panel,
      }}
    >
      <EyebrowText>{label}</EyebrowText>
      <Text
        className="text-lg font-extrabold"
        style={{ color: tokens.colors.textPrimary }}
      >
        {value}
      </Text>
    </View>
  );
}
