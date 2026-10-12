import { Icon } from "./Icon";
export function ChevronIcon({
  direction,
  color,
  size = 18,
}: {
  direction: "up" | "down";
  color?: string;
  size?: number;
}) {
  return (
    <Icon
      name={direction === "up" ? "chevron-up" : "chevron-down"}
      color={color}
      size={size}
    />
  );
}
