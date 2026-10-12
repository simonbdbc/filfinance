import { MasterDetail } from "@shared/ui/DetailPanel";
import { UIPressable } from "@shared/ui/primitives";
import { Icon } from "@shared/ui/Icon";
import { Text } from "@shared/ui/Text";
import { useEffect, useMemo, useState } from "react";
import { ActivityIndicator, View } from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type { CalendarDayItem, CalendarDayToday } from "@shared/api/contracts";
import {
  businessWeekDaysForLocalDay,
  defaultBusinessCalendarDayKey,
  formatLocalDateTime,
  formatLocalDayKey,
} from "@shared/lib/dateTime";
import {
  formatImportanceLabel,
  importanceBadgeVariant,
} from "@shared/lib/importance";
import { useThemeTokens } from "@shared/theme/theme";
import {
  Button,
  StatusBadge,
  Surface,
  SectionHeader,
  DataRow,
  PageHeader,
} from "@shared/ui";
import { MetaText, SubtleText } from "@shared/ui/Typography";
import { CalendarFiltersBlockContent } from "../components/CalendarFiltersBlock";
import { useCalendarDayToday } from "../hooks/useCalendarDayToday";
import { usePersistentCalendarFilters } from "../hooks/usePersistentCalendarFilters";
import {
  applyCalendarFilters,
  calendarFilterOptions,
  floorToMinute,
  nextCalendarSlotItems,
  sortCalendarItems,
} from "../lib/calendarFilters";

const calendarFiltersStorageKey = "filfinance.calendar.filters";
const fallbackCalendarEventDetail = "Aucun détail actuellement.";
const technicalForexFactoryReasoning =
  "Fallback deterministe depuis le calendrier ForexFactory date du jour cible.";

export function CalendarScreen() {
  const { authSession } = useAuth();
  const { server } = useAppRuntime();
  const [selectedDay, setSelectedDay] = useState(defaultBusinessCalendarDayKey);
  const [areFiltersVisible, setAreFiltersVisible] = useState(false);
  const minuteNow = useCurrentMinuteValue();
  const filterState = usePersistentCalendarFilters(calendarFiltersStorageKey);
  const businessDays = useMemo(
    () => businessWeekDaysForLocalDay(selectedDay),
    [selectedDay],
  );
  const calendar = useCalendarDayToday({
    authSession,
    isEnabled: Boolean(authSession),
    localDay: selectedDay,
    wsUrl: server.wsUrl,
  });
  const calendarItems = calendar.data?.items ?? [];
  const filterOptions = useMemo(
    () => calendarFilterOptions(calendarItems),
    [calendarItems],
  );
  const filteredItems = useMemo(
    () => applyCalendarFilters(calendarItems, filterState.filters),
    [calendarItems, filterState.filters],
  );
  const upcomingItems = useMemo(
    () => nextCalendarSlotItems(filteredItems, minuteNow),
    [filteredItems, minuteNow],
  );
  const sortedFilteredItems = useMemo(
    () => sortCalendarItems(filteredItems),
    [filteredItems],
  );

  return (
    <View className="w-full max-w-full min-w-0 gap-4">
      <PageHeader
        title={appViewDisplayNameByView.calendar}
        description="Événements économiques fusionnés du jour."
        action={
          <View className="max-w-full min-w-0 flex-row flex-wrap items-center justify-end gap-2">
            <Button disabled={calendar.isLoading} onPress={calendar.reload}>
              Rafraîchir
            </Button>
          </View>
        }
      />

      {calendar.error ? <CalendarError message={calendar.error} /> : null}
      <CalendarFiltersCard
        days={businessDays}
        isExpanded={areFiltersVisible}
        onExpandedChange={setAreFiltersVisible}
        onSelectDay={setSelectedDay}
        options={filterOptions}
        selectedDay={selectedDay}
        state={filterState}
      />
      <CalendarOverview data={calendar.data} isLoading={calendar.isLoading} />
      <CalendarUpcomingEventsCard
        hasData={Boolean(calendar.data)}
        isLoading={calendar.isLoading}
        items={upcomingItems}
      />
      <CalendarEventsCard
        hasData={Boolean(calendar.data)}
        isLoading={calendar.isLoading}
        items={sortedFilteredItems}
      />
    </View>
  );
}

function CalendarError({ message }: { message: string }) {
  const tokens = useThemeTokens();

  return (
    <Surface variant="danger">
      <SubtleText style={{ color: tokens.colors.dangerText }}>
        {message}
      </SubtleText>
    </Surface>
  );
}

function CalendarOverview({
  data,
  isLoading,
}: {
  data: CalendarDayToday | null;
  isLoading: boolean;
}) {
  const tokens = useThemeTokens();
  const description = data
    ? `${formatLocalDayKey(data.localDay, data.localDay)} - Événements : ${data.itemCount}, dont ${data.highItemCount} d’importance ${formatImportanceLabel("high").toLowerCase()}.`
    : "État du calendrier économique du jour.";

  return (
    <Surface>
      <View className="mb-3 w-full max-w-full min-w-0 flex-row items-start justify-between gap-3">
        <View className="min-w-0 flex-1 gap-1">
          <Text
            className="max-w-full min-w-0 text-sm font-bold uppercase tracking-[1px]"
            style={{ color: tokens.colors.textPrimary }}
          >
            Synthèse
          </Text>
          <Text
            className="max-w-full min-w-0 break-words text-sm leading-[19px]"
            style={{ color: tokens.colors.textPrimary }}
          >
            {description}
          </Text>
        </View>
      </View>

      {isLoading && !data ? (
        <ActivityIndicator size="small" />
      ) : (
        <View className="gap-1">
          {data?.summary?.title ? (
            <SubtleText className="font-semibold">
              {data.summary.title}
            </SubtleText>
          ) : null}
          {data?.summary?.summaryText ? (
            <SubtleText>{data.summary.summaryText}</SubtleText>
          ) : (
            <SubtleText>Aucune synthèse disponible pour le moment.</SubtleText>
          )}
        </View>
      )}
    </Surface>
  );
}

function CalendarFiltersCard({
  days,
  isExpanded,
  onExpandedChange,
  onSelectDay,
  options,
  selectedDay,
  state,
}: {
  days: string[];
  isExpanded: boolean;
  onExpandedChange: (isExpanded: boolean) => void;
  onSelectDay: (day: string) => void;
  options: Parameters<typeof CalendarFiltersBlockContent>[0]["options"];
  selectedDay: string;
  state: Parameters<typeof CalendarFiltersBlockContent>[0]["state"];
}) {
  return (
    <Surface>
      {/* <SectionHeader
        hasSeparator={false}
        title="FILTRES"
        description="Filtres partagés par les prochains événements et les événements fusionnés."
      /> */}
      <CalendarFiltersBlockContent
        days={days}
        isExpanded={isExpanded}
        onExpandedChange={onExpandedChange}
        onSelectDay={onSelectDay}
        options={options}
        selectedDay={selectedDay}
        state={state}
      />
    </Surface>
  );
}

function CalendarUpcomingEventsCard({
  hasData,
  isLoading,
  items,
}: {
  hasData: boolean;
  isLoading: boolean;
  items: CalendarDayItem[];
}) {
  return (
    <Surface>
      <SectionHeader
        hasSeparator={items.length > 0}
        title="Prochains événements"
        description={`Événements au prochain créneau disponible : ${items.length}.`}
      />

      {isLoading && !hasData ? (
        <ActivityIndicator size="small" />
      ) : items.length === 0 ? (
        <SubtleText>
          Aucun prochain événement ne correspond aux filtres actifs.
        </SubtleText>
      ) : (
        <View className="gap-0">
          {items.map((item) => (
            <CalendarEventRow item={item} key={item.id} />
          ))}
        </View>
      )}
    </Surface>
  );
}

function CalendarEventsCard({
  hasData,
  isLoading,
  items,
}: {
  hasData: boolean;
  isLoading: boolean;
  items: CalendarDayItem[];
}) {
  const [selectedId, setSelectedId] = useState<string | number | null>(null);
  const tokens = useThemeTokens();
  const selected = items.find((item) => item.id === selectedId) ?? null;
  return (
    <Surface>
      <SectionHeader
        title="Événements économiques"
        description={`Événements triés par horaire : ${items.length}.`}
        hasSeparator={false}
      />
      <MasterDetail
        title="Détail de l’événement"
        isOpen={!!selected}
        onClose={() => setSelectedId(null)}
        list={
          isLoading && !hasData ? (
            <ActivityIndicator accessibilityLabel="Chargement" />
          ) : items.length === 0 ? (
            <SubtleText>
              Aucun événement ne correspond aux filtres actifs.
            </SubtleText>
          ) : (
            <View>
              {items.map((item) => (
                <UIPressable
                  key={item.id}
                  accessibilityRole="button"
                  accessibilityState={{ selected: item.id === selectedId }}
                  onPress={() => setSelectedId(item.id)}
                  style={{
                    minHeight: 88,
                    padding: 16,
                    borderBottomWidth: 1,
                    borderColor: tokens.colors.borderSoft,
                    backgroundColor:
                      item.id === selectedId
                        ? tokens.colors.selection
                        : "transparent",
                    borderRadius: 6,
                    gap: 8,
                  }}
                >
                  <View className="flex-row flex-wrap items-center gap-2">
                    <MetaText>{formatEventTime(item.eventTime)}</MetaText>
                    <StatusBadge
                      variant={importanceBadgeVariant(item.finalImportance)}
                    >
                      {formatImportanceLabel(item.finalImportance)}
                    </StatusBadge>
                  </View>
                  <View className="flex-row gap-3">
                    <Text style={{ flex: 1 }}>
                      {item.translatedTitle ||
                        "Événement du calendrier non traduit"}
                    </Text>
                    <Icon name="chevron-right" />
                  </View>
                  <MetaText>
                    {[
                      item.country,
                      item.currency,
                      valueLabel("Actuel", item.actualValue),
                      valueLabel("Prévu", item.forecastValue),
                      valueLabel("Préc.", item.previousValue),
                    ]
                      .filter(Boolean)
                      .join(" · ")}
                  </MetaText>
                </UIPressable>
              ))}
            </View>
          )
        }
      >
        {selected ? <CalendarEventRow item={selected} /> : null}
      </MasterDetail>
    </Surface>
  );
}

function CalendarEventRow({ item }: { item: CalendarDayItem }) {
  const tokens = useThemeTokens();
  const title = item.translatedTitle || "Événement du calendrier non traduit";
  const marketLabels = [item.country, item.currency]
    .filter(Boolean)
    .join(" - ");
  const values = [
    valueLabel("Actuel", item.actualValue),
    valueLabel("Préc.", item.previousValue),
    valueLabel("Prévu", item.forecastValue),
  ].filter(Boolean);

  return (
    <DataRow className="gap-2">
      <View className="flex-row flex-wrap items-center gap-2">
        <StatusBadge
          textClassName="normal-case"
          variant={importanceBadgeVariant(item.finalImportance)}
        >
          {formatImportanceLabel(item.finalImportance)}
        </StatusBadge>
        <MetaText>{formatEventTime(item.eventTime)}</MetaText>
        {marketLabels ? (
          <StatusBadge variant="muted">{marketLabels}</StatusBadge>
        ) : null}
        <StatusBadge variant="trace">{item.category || "type n/a"}</StatusBadge>
        <StatusBadge variant="muted">
          confiance {formatConfidence(item.confidence)}
        </StatusBadge>
      </View>

      <Text
        className="max-w-full min-w-0 break-words text-sm font-bold leading-5"
        style={{ color: tokens.colors.textPrimary }}
      >
        {title}
      </Text>

      {values.length > 0 ? <MetaText>{values.join(" - ")}</MetaText> : null}
      <SubtleText>{calendarEventDetail(item.reasoning)}</SubtleText>
    </DataRow>
  );
}

function useCurrentMinuteValue() {
  const [minuteNow, setMinuteNow] = useState(() => floorToMinute(Date.now()));

  useEffect(() => {
    const timer = setInterval(() => {
      setMinuteNow(floorToMinute(Date.now()));
    }, 30_000);

    return () => clearInterval(timer);
  }, []);

  return minuteNow;
}

function formatConfidence(confidence: number | null) {
  if (confidence === null) return "n/a";
  return `${Math.round(confidence * 100)}%`;
}

function valueLabel(label: string, value: string | null) {
  return value ? `${label}: ${value}` : null;
}

function calendarEventDetail(reasoning: string | null) {
  const detail = reasoning?.trim();
  if (!detail || shouldHideCalendarEventReasoning(detail)) {
    return fallbackCalendarEventDetail;
  }
  return translateCalendarEventDetail(detail);
}

function shouldHideCalendarEventReasoning(reasoning: string) {
  return reasoning === technicalForexFactoryReasoning;
}

function translateCalendarEventDetail(reasoning: string) {
  const normalized = reasoning.toLowerCase();
  if (normalized.includes("actual significantly above forecast")) {
    return "Le chiffre publié est nettement supérieur aux prévisions, ce qui signale une demande plus forte.";
  }
  if (normalized.includes("actual above forecast")) {
    return "Le chiffre publié est supérieur aux prévisions.";
  }
  if (normalized.includes("actual significantly below forecast")) {
    return "Le chiffre publié est nettement inférieur aux prévisions, ce qui signale une demande plus faible.";
  }
  if (normalized.includes("actual below forecast")) {
    return "Le chiffre publié est inférieur aux prévisions.";
  }
  return reasoning;
}

function formatEventTime(value: string | null) {
  if (!value) {
    return "horaire n/a";
  }

  const parsed = new Date(value);
  if (!Number.isFinite(parsed.getTime())) {
    return "horaire n/a";
  }

  return formatLocalDateTime(value, "horaire n/a");
}
