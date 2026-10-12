import { KpiStrip } from "@shared/ui/Workspace";

import type { TranslatedNewsItem } from "@shared/api/contracts";

type LiveRuntimeStatsRowProps = {
  currentItem: TranslatedNewsItem | null;
  queuedCount: number;
  laggedCount: number | null;
};

export function LiveRuntimeStatsRow({
  currentItem,
  queuedCount,
  laggedCount,
}: LiveRuntimeStatsRowProps) {
  return (
    <KpiStrip
      items={[
        {
          label: "En lecture",
          tone: currentItem ? "info" : "muted",
          value: currentItem ? "1" : "0",
        },
        {
          label: "File",
          tone: queuedCount > 0 ? "trace" : "muted",
          value: queuedCount,
        },
        {
          label: "Retard",
          tone: (laggedCount ?? 0) > 0 ? "critical" : "muted",
          value: laggedCount ?? 0,
        },
      ]}
      minItemWidth={160}
    />
  );
}
