import { View, type SwitchProps } from "react-native";
import { UISwitch } from "./primitives";
import { useThemeTokens } from "../theme/theme";
export function Switch({
  accessibilityLabel = "Activer ou désactiver",
  ...props
}: Omit<SwitchProps, "trackColor" | "thumbColor">) {
  const t = useThemeTokens();
  return (
    <View style={{ minHeight: 44, minWidth: 52, justifyContent: "center" }}>
      <UISwitch
        {...props}
        accessibilityLabel={accessibilityLabel}
        style={{ width: 52, height: 44 }}
        thumbColor={t.colors.primaryText}
        trackColor={{
          false: t.colors.textSecondary,
          true: t.colors.primaryBackground,
        }}
      />
    </View>
  );
}
