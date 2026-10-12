import { Text } from "@shared/ui/Text";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ActivityIndicator, View } from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import {
  applyCalendarFilters,
  calendarFilterOptions,
  CalendarFiltersBlockContent,
  sortCalendarItems,
  usePersistentCalendarFilters,
} from "@features/calendar";
import { useAuth } from "@features/auth";
import type {
  AdminCalendarDayToday,
  CalendarDayAlert,
  CalendarDayItem,
  CalendarDayItemAudio,
} from "@shared/api/contracts";
import {
  businessWeekDaysForLocalDay,
  defaultBusinessCalendarDayKey,
  formatLocalDateTimeWithRaw,
} from "@shared/lib/dateTime";
import {
  formatImportanceLabel,
  importanceBadgeVariant,
} from "@shared/lib/importance";
import { useThemeTokens } from "@shared/theme/theme";
import {
  AccessDeniedCard,
  Button,
  StatusBadge,
  MetaText,
  SubtleText,
  UserProfileLoadingCard,
} from "@shared/ui";
import {
  Grid,
  KpiStrip,
  DataRow,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
} from "@shared/ui/Workspace";
import { loadAdminDailySummaryToday } from "../services/adminDailySummaryApi";

const calendarHistoryFiltersStorageKey =
  "filfinance.admin.calendarHistory.filters";

export function AdminCalendarHistoryScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const [selectedDay, setSelectedDay] = useState(defaultBusinessCalendarDayKey);
  const [isFilterPanelOpen, setIsFilterPanelOpen] = useState(false);
  const [reloadVersion, setReloadVersion] = useState(0);
  const [snapshot, setSnapshot] = useState<AdminCalendarDayToday | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const filterState = usePersistentCalendarFilters(
    calendarHistoryFiltersStorageKey,
  );
  const businessDays = useMemo(
    () => businessWeekDaysForLocalDay(selectedDay),
    [selectedDay],
  );

  useEffect(() => {
    if (!canAccessAdmin || !authSession) {
      return;
    }

    let cancelled = false;
    setIsLoading(true);
    setError(null);

    void loadAdminDailySummaryToday({
      authSession,
      localDay: selectedDay,
      wsUrl: server.wsUrl,
    })
      .then((payload) => {
        if (!cancelled) {
          setSnapshot(payload);
        }
      })
      .catch((loadError: unknown) => {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Erreur calendrier admin.",
          );
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [authSession, canAccessAdmin, reloadVersion, selectedDay, server.wsUrl]);

  const reload = useCallback(() => {
    setReloadVersion((version) => version + 1);
  }, []);
  const items = snapshot?.items ?? [];
  const filterOptions = useMemo(() => calendarFilterOptions(items), [items]);
  const filteredItems = useMemo(
    () => sortCalendarItems(applyCalendarFilters(items, filterState.filters)),
    [filterState.filters, items],
  );
  const audiosByItemId = useMemo(
    () => groupAudiosByItemId(snapshot?.audios ?? []),
    [snapshot?.audios],
  );
  const alertsByItemId = useMemo(
    () => groupAlertsByItemId(snapshot?.alerts ?? []),
    [snapshot?.alerts],
  );

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminCalendarHistory}
      description="Historique détaillé des événements fusionnés du calendrier."
      actions={
        <Button disabled={isLoading} onPress={reload}>
          Rafraîchir
        </Button>
      }
    >
      <KpiStrip
        items={[
          { label: "Jour", value: selectedDay, tone: "info" },
          {
            label: "Événements",
            value: items.length,
            tone: items.length ? "trace" : "muted",
          },
          {
            label: "Audios",
            value: snapshot?.audios.length ?? 0,
            tone: snapshot?.audios.length ? "info" : "muted",
          },
          {
            label: "Alertes",
            value: snapshot?.alerts.length ?? 0,
            tone: snapshot?.alerts.length ? "trace" : "muted",
          },
        ]}
      />

      <Grid direction="vertical">
        <SectionPanel
          title="FILTRES"
          description="Filtres appliqués à tous les événements affichés."
        >
          <CalendarFiltersBlockContent
            days={businessDays}
            isExpanded={isFilterPanelOpen}
            onExpandedChange={setIsFilterPanelOpen}
            onSelectDay={setSelectedDay}
            options={filterOptions}
            selectedDay={selectedDay}
            state={filterState}
          />
        </SectionPanel>

        <SectionPanel
          title="Événements fusionnés"
          description={`Événements après filtrage : ${filteredItems.length}, sans le bloc des prochains événements.`}
        >
          <Grid direction="vertical">
            {error ? <SubtleText>{error}</SubtleText> : null}
            {isLoading && !snapshot ? <ActivityIndicator size="small" /> : null}
            {!isLoading && filteredItems.length === 0 ? (
              <SubtleText>
                Aucun événement ne correspond aux filtres actifs.
              </SubtleText>
            ) : null}
            {filteredItems.map((item) => (
              <CalendarHistoryEventRow
                alerts={alertsByItemId.get(item.id) ?? []}
                audios={audiosByItemId.get(item.id) ?? []}
                item={item}
                key={item.id}
              />
            ))}
          </Grid>
        </SectionPanel>
      </Grid>
    </WorkspacePage>
  );
}

function CalendarHistoryEventRow({
  alerts,
  audios,
  item,
}: {
  alerts: CalendarDayAlert[];
  audios: CalendarDayItemAudio[];
  item: CalendarDayItem;
}) {
  const tokens = useThemeTokens();
  const values = [
    fieldLabel("Actuel", item.actualValue),
    fieldLabel("Précédent", item.previousValue),
    fieldLabel("Prévision", item.forecastValue),
  ].filter(Boolean);
  const market =
    [item.country, item.currency].filter(Boolean).join(" - ") ||
    "localisation n/a";

  return (
    <DataRow>
      <View className="flex-row flex-wrap items-center gap-2">
        <StatusBadge
          textClassName="normal-case"
          variant={importanceBadgeVariant(item.finalImportance)}
        >
          finale {formatImportanceLabel(item.finalImportance)}
        </StatusBadge>
        {item.sourceImportance ? (
          <StateIndicator tone="muted">
            source{" "}
            {formatImportanceLabel(
              item.sourceImportance,
              item.sourceImportance,
            )}
          </StateIndicator>
        ) : null}
        {item.aiImportance ? (
          <StateIndicator tone="trace">
            IA {formatImportanceLabel(item.aiImportance, item.aiImportance)}
          </StateIndicator>
        ) : null}
        <StateIndicator tone="muted">
          confiance {formatConfidence(item.confidence)}
        </StateIndicator>
      </View>

      <View className="gap-1">
        <Text
          className="text-sm font-bold leading-5"
          style={{ color: tokens.colors.textPrimary }}
        >
          {item.translatedTitle || item.sourceTitle}
        </Text>
        <SubtleText>Source EN : {item.sourceTitle || "n/a"}</SubtleText>
        <SubtleText>TTS : {item.ttsText || "n/a"}</SubtleText>
      </View>

      <View className="flex-row flex-wrap gap-2">
        <StateIndicator tone="muted">id {item.id}</StateIndicator>
        <StateIndicator tone="muted">{item.localDay}</StateIndicator>
        <StateIndicator tone="trace">
          {formatEventTime(item.eventTime)}
        </StateIndicator>
        <StateIndicator tone="muted">{market}</StateIndicator>
        <StateIndicator tone="trace">
          {item.category || "type n/a"}
        </StateIndicator>
      </View>

      {values.length > 0 ? <MetaText>{values.join(" • ")}</MetaText> : null}
      {item.reasoning ? (
        <SubtleText>Explication : {item.reasoning}</SubtleText>
      ) : null}
      <MetaText>content_hash={item.contentHash}</MetaText>
      <MetaText>
        created={formatLocalDateTimeWithRaw(item.createdAt)} • updated=
        {formatLocalDateTimeWithRaw(item.updatedAt)}
      </MetaText>
      <SubtleText>
        sourcesJson: {formatSourcesJson(item.sourcesJson)}
      </SubtleText>

      {audios.length > 0 ? <RelatedAudios audios={audios} /> : null}
      {alerts.length > 0 ? <RelatedAlerts alerts={alerts} /> : null}
    </DataRow>
  );
}

function RelatedAudios({ audios }: { audios: CalendarDayItemAudio[] }) {
  return (
    <View className="gap-1">
      <MetaText>Audios liés</MetaText>
      {audios.map((audio) => (
        <SubtleText key={audio.id}>
          #{audio.id} • item={audio.calendarItemId} • {audio.audioKind} •{" "}
          {audio.status} • stream={audio.streamId ?? "n/a"} • playlist=
          {audio.playlistUrl ?? "n/a"} • error={audio.errorMessage ?? "n/a"} •
          created={formatLocalDateTimeWithRaw(audio.createdAt)} • updated=
          {formatLocalDateTimeWithRaw(audio.updatedAt)}
        </SubtleText>
      ))}
    </View>
  );
}

function RelatedAlerts({ alerts }: { alerts: CalendarDayAlert[] }) {
  return (
    <View className="gap-1">
      <MetaText>Alertes liées</MetaText>
      {alerts.map((alert) => (
        <SubtleText key={alert.id}>
          #{alert.id} • item={alert.calendarItemId} • offset=
          {alert.alertOffsetSecs}s • {alert.status} • scheduled=
          {formatLocalDateTimeWithRaw(alert.scheduledAt)} • stream=
          {alert.streamId ?? "n/a"} • playlist={alert.playlistUrl ?? "n/a"} •
          published={formatOptionalDate(alert.publishedAt)} • expires=
          {formatOptionalDate(alert.expiresAt)} • error=
          {alert.errorMessage ?? "n/a"} • created=
          {formatLocalDateTimeWithRaw(alert.createdAt)} • updated=
          {formatLocalDateTimeWithRaw(alert.updatedAt)}
        </SubtleText>
      ))}
    </View>
  );
}

function groupAudiosByItemId(audios: CalendarDayItemAudio[]) {
  const grouped = new Map<number, CalendarDayItemAudio[]>();
  audios.forEach((audio) => {
    grouped.set(audio.calendarItemId, [
      ...(grouped.get(audio.calendarItemId) ?? []),
      audio,
    ]);
  });
  return grouped;
}

function groupAlertsByItemId(alerts: CalendarDayAlert[]) {
  const grouped = new Map<number, CalendarDayAlert[]>();
  alerts.forEach((alert) => {
    grouped.set(alert.calendarItemId, [
      ...(grouped.get(alert.calendarItemId) ?? []),
      alert,
    ]);
  });
  return grouped;
}

function fieldLabel(label: string, value: string | null) {
  return value ? `${label}: ${value}` : null;
}

function formatConfidence(confidence: number | null) {
  if (confidence === null) {
    return "n/a";
  }
  return `${Math.round(confidence * 100)}%`;
}

function formatEventTime(value: string | null) {
  return value ? formatLocalDateTimeWithRaw(value) : "horaire n/a";
}

function formatOptionalDate(value: string | null) {
  return value ? formatLocalDateTimeWithRaw(value) : "n/a";
}

function formatSourcesJson(value: string) {
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value || "n/a";
  }
}
