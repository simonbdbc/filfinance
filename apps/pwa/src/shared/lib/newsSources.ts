const sourceDisplayLabelByName: Record<string, string> = {
  financial_juice: "Financial Juice",
  financial_juice_browser: "Financial Juice B",
  x: "Réseau social X",
};

export function sourceDisplayLabel(sourceName: string) {
  return sourceDisplayLabelByName[sourceName] ?? sourceName;
}
