import { Text } from "@shared/ui/Text";

import { DataRow, SectionPanel } from "@shared/ui/Workspace";

import type { TranslatedNewsItem } from "@shared/api/contracts";
import {
  formatLocalDateTimeWithRaw,
  formatLocalTime,
} from "@shared/lib/dateTime";
import { filterHistoryItemsByLocalDayKey } from "@shared/lib/newsHistory";
import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { SubtleText } from "@shared/ui/Typography";

type TestRunHistoryCardProps = {
  items: TranslatedNewsItem[];
  isLoading: boolean;
  currentLocalDayKey: string;
  previousLocalDayKey: string;
  isCollapsed: boolean;
  onCollapsedChange: (isCollapsed: boolean) => void;
  onReplayAudio: (items: TranslatedNewsItem[], historyLabel: string) => void;
};

export function TestRunHistoryCard({
  items,
  isLoading,
  currentLocalDayKey,
  previousLocalDayKey,
  isCollapsed,
  onCollapsedChange,
  onReplayAudio,
}: TestRunHistoryCardProps) {
  const tokens = useThemeTokens();
  const currentDayHistory = filterHistoryItemsByLocalDayKey(
    items,
    currentLocalDayKey,
  );
  const previousDayHistory = filterHistoryItemsByLocalDayKey(
    items,
    previousLocalDayKey,
  );
  const primaryHistory =
    currentDayHistory.length > 0 ? currentDayHistory : previousDayHistory;
  const primaryDayKey =
    currentDayHistory.length > 0
      ? currentLocalDayKey
      : previousDayHistory.length > 0
        ? previousLocalDayKey
        : currentLocalDayKey;
  const latestItem = items[0] || null;
  const historyDescription = `${primaryDayKey}${
    currentDayHistory.length > 0 && previousDayHistory.length > 0
      ? ` • J-1=${previousLocalDayKey}`
      : !currentDayHistory.length && previousDayHistory.length > 0
        ? " • dernier jour disponible"
        : ""
  }${latestItem ? ` • dernier=${latestItem.test_run_id || latestItem.id}` : " • aucun élément"}`;

  return (
    <SectionPanel
      title="Historique journalier des tests"
      description={historyDescription}
      action={
        <Button
          className="min-w-[104px]"
          onPress={() => onCollapsedChange(!isCollapsed)}
        >
          {isCollapsed ? "Afficher" : "Réduire"}
        </Button>
      }
    >
      {!isCollapsed ? (
        isLoading ? (
          <SubtleText>Chargement de l’historique des tests…</SubtleText>
        ) : items.length === 0 ? (
          <SubtleText>Aucun test reçu pour aujourd’hui ou hier.</SubtleText>
        ) : (
          <>
            <Button
              className="w-full"
              disabled={primaryHistory.length === 0}
              textClassName="text-center"
              onPress={() => {
                onReplayAudio(
                  primaryHistory,
                  currentDayHistory.length > 0
                    ? "l’historique des tests du jour"
                    : "le dernier jour de tests disponible",
                );
              }}
            >
              {currentDayHistory.length > 0
                ? "Réécouter les tests audio du jour"
                : "Réécouter les tests audio du dernier jour"}
            </Button>
            {items.map((item) => (
              <DataRow key={`${item.id}-${item.updated_at}`}>
                <Text
                  className="text-sm font-semibold leading-[18px]"
                  style={{ color: tokens.colors.textSecondary }}
                >
                  {item.source_title}
                </Text>
                <Text
                  className="text-sm font-bold leading-5"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {item.translated_title}
                </Text>
                <Text
                  className="text-sm"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {formatLocalTime(item.timestamp || item.created_at)} •{" "}
                  {item.source} • {item.translation_mode} •{" "}
                  {formatLocalDateTimeWithRaw(
                    item.timestamp || item.created_at,
                  )}
                  {item.news_level ? ` • level=${item.news_level}` : ""}
                  {item.test_run_id ? ` • ${item.test_run_id}` : ""}
                </Text>
              </DataRow>
            ))}
          </>
        )
      ) : null}
    </SectionPanel>
  );
}
