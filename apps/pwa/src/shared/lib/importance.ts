export type ImportanceLevel = "high" | "medium" | "low";

export type ImportanceBadgeVariant =
  | "critical"
  | "dark"
  | "info"
  | "muted"
  | "trace";

const importanceLabelByLevel: Record<ImportanceLevel, string> = {
  high: "Élevée",
  low: "Faible",
  medium: "Moyenne",
};

export function normalizeImportance(
  value: string | null | undefined,
): ImportanceLevel | null {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "high" || normalized === "medium" || normalized === "low") {
    return normalized;
  }
  return null;
}

export function formatImportanceLabel(
  value: string | null | undefined,
  fallback = "n/a",
) {
  const level = normalizeImportance(value);
  return level ? importanceLabelByLevel[level] : fallback;
}

export function importanceSortValue(value: string | null | undefined) {
  const level = normalizeImportance(value);
  if (level === "high") return 3;
  if (level === "medium") return 2;
  if (level === "low") return 1;
  return 0;
}

export function importanceBadgeVariant(
  value: string | null | undefined,
): ImportanceBadgeVariant {
  const level = normalizeImportance(value);
  if (level === "high") return "critical";
  if (level === "medium") return "trace";
  return "muted";
}
