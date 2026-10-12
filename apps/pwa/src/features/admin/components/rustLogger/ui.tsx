import type { ReactNode } from "react";
import { Button } from "@shared/ui/Button";
export function Action({
  children,
  onPress,
  disabled = false,
  label,
  primary,
}: {
  children: ReactNode;
  onPress: () => void;
  disabled?: boolean;
  primary?: boolean;
  label?: string;
}) {
  return (
    <Button
      disabled={disabled}
      accessibilityLabel={label}
      variant={primary ? "primary" : "secondary"}
      onPress={onPress}
    >
      {children}
    </Button>
  );
}
