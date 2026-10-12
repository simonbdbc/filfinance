import Feather from "@expo/vector-icons/Feather";
import { createContext, useContext, type ComponentProps } from "react";
import { useThemeTokens } from "../theme/theme";
export const IconColorContext = createContext<string | undefined>(undefined);
export type IconName = ComponentProps<typeof Feather>["name"];
export function Icon({
  name,
  size = 22,
  color,
}: {
  name: IconName;
  size?: number;
  color?: string;
}) {
  const t = useThemeTokens();
  const inheritedColor = useContext(IconColorContext);
  return (
    <Feather
      name={name}
      size={size}
      color={color ?? inheritedColor ?? t.colors.textPrimary}
      accessible={false}
      aria-hidden={true}
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
    />
  );
}
