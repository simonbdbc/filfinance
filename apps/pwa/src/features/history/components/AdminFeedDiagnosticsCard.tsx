import { Text } from "@shared/ui/Text";
import { View } from "react-native";

import {
  Grid,
  KpiStrip,
  DataRow,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
} from "@shared/ui/Workspace";

import type {
  NewsTruncationAudit,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import {
  formatLocalDateTimeWithRaw,
  formatLocalTime,
} from "@shared/lib/dateTime";
import {
  filterHistoryItemsByLocalDayKey,
  newsLevelContainsActive,
} from "@shared/lib/newsHistory";
import { useThemeTokens } from "@shared/theme/theme";
import { StatusBadge } from "@shared/ui/StatusBadge";
import { Button } from "@shared/ui/Button";
import { ChevronIcon } from "@shared/ui/ChevronIcon";
import { MetaText, SubtleText } from "@shared/ui/Typography";

type AdminFeedDiagnosticsCardProps = {
  currentLocalDayKey: string;
  isLoading: boolean;
  isPreviousDayVisible: boolean;
  isTruncationAuditLoading: boolean;
  items: TranslatedNewsItem[];
  onPreviousDayVisibilityChange: (isVisible: boolean) => void;
  onReloadTruncationAudits: () => void;
  previousLocalDayKey: string;
  title: string;
  truncationAuditError: string | null;
  truncationAudits: NewsTruncationAudit[];
};

export function AdminFeedDiagnosticsCard({
  currentLocalDayKey,
  isLoading,
  isPreviousDayVisible,
  isTruncationAuditLoading,
  items,
  onPreviousDayVisibilityChange,
  onReloadTruncationAudits,
  previousLocalDayKey,
  title,
  truncationAuditError,
  truncationAudits,
}: AdminFeedDiagnosticsCardProps) {
  const currentDayHistory = filterHistoryItemsByLocalDayKey(
    items,
    currentLocalDayKey,
  );
  const previousDayHistory = filterHistoryItemsByLocalDayKey(
    items,
    previousLocalDayKey,
  );
  const hasCurrentDayHistory = currentDayHistory.length > 0;
  const primaryHistory = hasCurrentDayHistory
    ? currentDayHistory
    : previousDayHistory;
  const secondaryHistory = hasCurrentDayHistory ? previousDayHistory : [];
  const primaryDayKey = hasCurrentDayHistory
    ? currentLocalDayKey
    : previousDayHistory.length > 0
      ? previousLocalDayKey
      : currentLocalDayKey;
  const historyDescription = `${primaryDayKey}${
    hasCurrentDayHistory && previousDayHistory.length > 0
      ? ` • J-1=${previousLocalDayKey}`
      : !hasCurrentDayHistory && previousDayHistory.length > 0
        ? " • dernier jour disponible"
        : ""
  } • flux réel \`financial_juice\``;
  const visibleHistory = isPreviousDayVisible
    ? [...primaryHistory, ...secondaryHistory]
    : primaryHistory;
  const activeVisibleTotal = visibleHistory.filter((item) =>
    newsLevelContainsActive(item.news_level),
  ).length;

  return (
    <WorkspacePage
      title={title}
      description="Diagnostic du fil d’actualités par jour, source, traduction et niveau."
    >
      <KpiStrip
        items={[
          {
            label: "Jour",
            value: primaryDayKey,
            tone: "info",
          },
          {
            label: "Éléments visibles",
            value: visibleHistory.length,
            tone: visibleHistory.length > 0 ? "trace" : "muted",
          },
          {
            label: "Actualités d’importance élevée",
            value: activeVisibleTotal,
            tone: activeVisibleTotal > 0 ? "critical" : "muted",
          },
          {
            label: "Troncatures",
            value: truncationAudits.length,
            tone: truncationAudits.length > 0 ? "critical" : "muted",
          },
        ]}
      />

      <Grid direction="vertical">
        <SectionPanel
          title="Diagnostic journalier"
          description={historyDescription}
        >
          <Grid direction="vertical">
            {isLoading ? (
              <SubtleText>Chargement de l’historique journalier…</SubtleText>
            ) : visibleHistory.length === 0 ? (
              <EmptyHistoryState
                canShowPreviousDay={secondaryHistory.length > 0}
                onShowPreviousDay={() => onPreviousDayVisibilityChange(true)}
              />
            ) : (
              <>
                {visibleHistory.map((item) => (
                  <HistoryDiagnosticRow
                    key={`${item.id}-${item.updated_at}`}
                    item={item}
                  />
                ))}
                {secondaryHistory.length > 0 ? (
                  <Button
                    className="w-full"
                    textClassName="text-center"
                    onPress={() =>
                      onPreviousDayVisibilityChange(!isPreviousDayVisible)
                    }
                    suffix={
                      <ChevronIcon
                        direction={isPreviousDayVisible ? "up" : "down"}
                        size={14}
                      />
                    }
                  >
                    {isPreviousDayVisible ? "Voir moins" : "Voir plus"}
                  </Button>
                ) : null}
              </>
            )}
          </Grid>
        </SectionPanel>

        <SectionPanel
          title="Audits de troncature en amont"
          description="FinancialJuice a déjà envoyé un contenu signalé comme tronqué avant sa traduction."
          action={
            <Button
              disabled={isTruncationAuditLoading}
              onPress={onReloadTruncationAudits}
            >
              Actualiser
            </Button>
          }
        >
          <Grid direction="vertical">
            {truncationAuditError ? (
              <SubtleText>{truncationAuditError}</SubtleText>
            ) : isTruncationAuditLoading ? (
              <SubtleText>Chargement des audits de troncature…</SubtleText>
            ) : truncationAudits.length === 0 ? (
              <SubtleText>
                Aucune troncature détectée récemment en amont.
              </SubtleText>
            ) : (
              truncationAudits.map((audit) => (
                <TruncationAuditRow audit={audit} key={audit.id} />
              ))
            )}
          </Grid>
        </SectionPanel>
      </Grid>
    </WorkspacePage>
  );
}

function EmptyHistoryState({
  canShowPreviousDay,
  onShowPreviousDay,
}: {
  canShowPreviousDay: boolean;
  onShowPreviousDay: () => void;
}) {
  return (
    <>
      <SubtleText>Aucune actualité disponible pour aujourd’hui ou hier.</SubtleText>
      {canShowPreviousDay ? (
        <Button
          className="w-full"
          textClassName="text-center"
          onPress={onShowPreviousDay}
          suffix={<ChevronIcon direction="down" size={14} />}
        >
          Voir plus
        </Button>
      ) : null}
    </>
  );
}

function TruncationAuditRow({ audit }: { audit: NewsTruncationAudit }) {
  const tokens = useThemeTokens();

  return (
    <DataRow
      style={{
        backgroundColor: tokens.colors.surface,
      }}
    >
      <StateIndicator tone="critical">marker {audit.marker}</StateIndicator>
      <View className="gap-1">
        <Text
          className="text-sm font-semibold leading-[18px]"
          style={{ color: tokens.colors.textSecondary }}
        >
          {audit.source_title}
        </Text>
        <Text
          className="text-sm font-bold leading-5"
          style={{ color: tokens.colors.textPrimary }}
        >
          {audit.field_excerpt}
        </Text>
        <Text className="text-sm" style={{ color: tokens.colors.textPrimary }}>
          {audit.source} • {audit.field_name} • {audit.raw_payload_ref ?? "n/a"}{" "}
          • {formatLocalDateTimeWithRaw(audit.created_at)}
        </Text>
      </View>
    </DataRow>
  );
}

function HistoryDiagnosticRow({ item }: { item: TranslatedNewsItem }) {
  const tokens = useThemeTokens();
  const isActiveNews = newsLevelContainsActive(item.news_level);
  const levelBadge = historyNewsLevelBadge(item.news_level);
  const textColor = isActiveNews
    ? tokens.colors.dangerText
    : tokens.colors.textPrimary;
  const metaColor = isActiveNews
    ? tokens.colors.dangerText
    : tokens.colors.textPrimary;

  return (
    <DataRow
      style={
        isActiveNews
          ? {
              backgroundColor: tokens.colors.surface,
            }
          : undefined
      }
    >
      <View className="gap-1">
        <Text className="text-sm" style={{ color: metaColor }}>
          {formatLocalTime(item.timestamp || item.created_at)} • {item.source} •{" "}
          {item.translation_mode} •{" "}
          {formatLocalDateTimeWithRaw(item.timestamp || item.created_at)}
          {item.news_level ? ` • level=${item.news_level}` : ""}
        </Text>
        <StatusBadge
          className="shrink-0"
          textClassName="normal-case tracking-normal"
          variant={levelBadge.variant}
        >
          {levelBadge.label}
        </StatusBadge>
        <Text
          className="text-sm font-semibold leading-[18px]"
          style={{
            color: isActiveNews
              ? tokens.colors.dangerText
              : tokens.colors.textSecondary,
          }}
        >
          Anglais
        </Text>
        <Text
          className="text-sm font-bold leading-5"
          style={{ color: textColor }}
        >
          {item.source_title}
        </Text>
        {item.source_body ? (
          <Text className="text-sm leading-[18px]" style={{ color: metaColor }}>
            {item.source_body}
          </Text>
        ) : (
          <MetaText>Contenu anglais n/a</MetaText>
        )}

        <Text
          className="pt-1 text-sm font-semibold leading-[18px]"
          style={{
            color: isActiveNews
              ? tokens.colors.dangerText
              : tokens.colors.textSecondary,
          }}
        >
          Français
        </Text>
        <Text
          className="text-sm font-bold leading-5"
          style={{ color: textColor }}
        >
          {item.translated_title}
        </Text>
        {item.translated_body ? (
          <Text className="text-sm leading-[18px]" style={{ color: metaColor }}>
            {item.translated_body}
          </Text>
        ) : (
          <MetaText>Contenu français n/a</MetaText>
        )}

        <Text
          className="pt-1 text-sm font-semibold leading-[18px]"
          style={{
            color: isActiveNews
              ? tokens.colors.dangerText
              : tokens.colors.textSecondary,
          }}
        >
          TTS
        </Text>
        <Text className="text-sm leading-[18px]" style={{ color: metaColor }}>
          {item.narration_text || "n/a"}
        </Text>
      </View>
    </DataRow>
  );
}

type HistoryNewsLevelBadge = {
  label: string;
  variant: "critical" | "muted" | "trace";
};

function historyNewsLevelBadge(
  newsLevel?: string | null,
): HistoryNewsLevelBadge {
  if (newsLevelContainsActive(newsLevel)) {
    return { label: "active -> Élevée", variant: "critical" };
  }

  if (
    String(newsLevel ?? "")
      .trim()
      .toLowerCase() === "news-general"
  ) {
    return { label: "news-general -> Général", variant: "trace" };
  }

  return { label: "none -> Faible", variant: "muted" };
}
