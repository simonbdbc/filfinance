import type { AuthSessionClient } from "@shared/api/authSession";
import { useEffect, useRef, useState } from "react";
import { Linking, View, useWindowDimensions } from "react-native";
import type {
  PublicSourceStatus,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import { canAccessAdmin, type AppRole } from "@shared/auth/roles";
import { formatLocalDayKey, formatLocalTime } from "@shared/lib/dateTime";
import {
  filterHistoryItemsByLocalDayKey,
  itemTimestampValue,
  newsLevelContainsActive,
} from "@shared/lib/newsHistory";
import { sourceDisplayLabel } from "@shared/lib/newsSources";
import { useThemeTokens } from "@shared/theme";
import { Text, headingStyle } from "@shared/ui/Text";
import { Button, Surface, PageHeader, StatusBadge } from "@shared/ui";
import { UIPressable } from "@shared/ui/primitives";
import { Icon } from "@shared/ui/Icon";
import { MasterDetail, DialogPanel } from "@shared/ui/DetailPanel";
import { AsyncState } from "@shared/ui/AsyncState";
import { loadTranslatedNewsDetail } from "../services/loadTranslatedNewsDetail";

type NewsFeedCardProps = {
  authSession: AuthSessionClient | null;
  title: string;
  items: TranslatedNewsItem[];
  isLoading: boolean;
  currentLocalDayKey: string;
  previousLocalDayKey: string;
  isPreviousDayVisible: boolean;
  onPreviousDayVisibilityChange: (isVisible: boolean) => void;
  newsSourceStatuses: PublicSourceStatus[];
  role: AppRole | null;
  wsUrl: string;
};

export function NewsFeedCard(props: NewsFeedCardProps) {
  const t = useThemeTokens();
  const compact = useWindowDimensions().width < 768;
  const [selected, setSelected] = useState<TranslatedNewsItem | null>(null);
  const [detail, setDetail] = useState<TranslatedNewsItem | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const request = useRef(0);
  const day = props.isPreviousDayVisible
    ? props.previousLocalDayKey
    : props.currentLocalDayKey;
  const items = filterHistoryItemsByLocalDayKey(props.items, day);
  // Invalider les réponses tardives à la déconnexion ou au changement de jour.
  useEffect(() => {
    request.current++;
    setSelected(null);
    setDetail(null);
  }, [day, props.authSession]);
  useEffect(
    () => () => {
      request.current++;
    },
    [],
  );
  async function select(item: TranslatedNewsItem) {
    const id = ++request.current;
    setSelected(item);
    setDetail(null);
    setDetailError(null);
    setLoadingDetail(true);
    try {
      const result = await loadTranslatedNewsDetail({
        authSession: props.authSession,
        id: item.id,
        wsUrl: props.wsUrl,
      });
      if (request.current === id) setDetail(result);
    } catch {
      if (request.current === id)
        setDetailError("Impossible de charger le détail complet.");
    } finally {
      if (request.current === id) setLoadingDetail(false);
    }
  }
  const current = detail ?? selected;
  const sourceUrl =
    current?.url && /^https?:\/\//i.test(current.url) ? current.url : null;
  const visibleSources = [
    ...(canAccessAdmin(props.role)
      ? props.newsSourceStatuses
      : props.newsSourceStatuses.filter((s) => s.enabled)),
  ].sort((a, b) => a.source.localeCompare(b.source));
  return (
    <View className="w-full min-w-0 gap-6">
      <Surface unframed={compact}>
        <View className="mb-4 flex-row flex-wrap items-center justify-between gap-3">
          <View className="flex-row flex-wrap gap-2">
            <Button
              selected={!props.isPreviousDayVisible}
              onPress={() => props.onPreviousDayVisibilityChange(false)}
            >
              Aujourd’hui
            </Button>
            <Button
              selected={props.isPreviousDayVisible}
              onPress={() => props.onPreviousDayVisibilityChange(true)}
            >
              Hier
            </Button>
          </View>
          <Button
            onPress={() => setSourcesOpen(true)}
            prefix={<Icon name="settings" />}
          >
            Sources
          </Button>
        </View>
        <MasterDetail
          title="Détail de l’actualité"
          isOpen={!!selected}
          onClose={() => setSelected(null)}
          list={
            props.isLoading ? (
              <AsyncState loading />
            ) : items.length === 0 ? (
              <AsyncState empty="Aucune actualité disponible pour cette journée." />
            ) : (
              <View>
                {items.map((item) => (
                  <UIPressable
                    key={item.id}
                    testID={`news-${item.id}`}
                    accessibilityRole="button"
                    accessibilityLabel={`${formatLocalTime(itemTimestampValue(item))} · ${item.translated_title || item.source_title} · ${sourceDisplayLabel(item.source)}`}
                    aria-expanded={selected?.id === item.id}
                    onPress={() => void select(item)}
                    style={{
                      minHeight: 88,
                      paddingVertical: 16,
                      paddingHorizontal: compact ? 4 : 16,
                      gap: 12,
                      flexDirection: "row",
                      borderBottomWidth: 1,
                      borderColor: t.colors.borderSoft,
                      borderLeftWidth: 3,
                      borderLeftColor:
                        selected?.id === item.id
                          ? t.colors.accentDecorative
                          : "transparent",
                      backgroundColor:
                        selected?.id === item.id
                          ? compact
                            ? t.colors.surface
                            : t.colors.selection
                          : "transparent",
                      borderRadius: 6,
                    }}
                  >
                    <Text
                      style={{ fontWeight: "700", fontSize: 14, width: 44 }}
                    >
                      {formatLocalTime(itemTimestampValue(item))}
                    </Text>
                    <View style={{ flex: 1, minWidth: 0, gap: 4 }}>
                      <Text>{item.translated_title || item.source_title}</Text>
                      <Text style={{ fontSize: 14 }}>
                        {sourceDisplayLabel(item.source)}
                      </Text>
                      {newsLevelContainsActive(item.news_level) ? (
                        <StatusBadge variant="critical">
                          Importance élevée
                        </StatusBadge>
                      ) : null}
                    </View>
                    <Icon name="chevron-right" size={18} />
                  </UIPressable>
                ))}
              </View>
            )
          }
        >
          {current ? (
            <View className="gap-6">
              <Text style={{ fontSize: 14 }}>
                {sourceDisplayLabel(current.source)} ·{" "}
                {formatLocalTime(itemTimestampValue(current))}
              </Text>
              <Text style={headingStyle(t, 28)}>
                {current.translated_title || current.source_title}
              </Text>
              {loadingDetail || detailError ? (
                <AsyncState
                  loading={loadingDetail}
                  error={detailError}
                  onRetry={() => selected && void select(selected)}
                />
              ) : (
                <Text>
                  {current.translated_body?.trim() ||
                    "Aucun détail supplémentaire disponible."}
                </Text>
              )}
              {sourceUrl ? (
                <Button
                  onPress={() =>
                    void Linking.openURL(sourceUrl).catch(() =>
                      setDetailError("Impossible d’ouvrir la source."),
                    )
                  }
                  suffix={<Icon name="external-link" />}
                >
                  Consulter la source
                </Button>
              ) : null}
            </View>
          ) : null}
        </MasterDetail>
      </Surface>
      <DialogPanel
        title="Sources d’actualités"
        isOpen={sourcesOpen}
        onClose={() => setSourcesOpen(false)}
      >
        {props.role === "listener" ? (
          <Text>
            {props.newsSourceStatuses.some(
              (s) => s.enabled || s.status === "active",
            )
              ? "Source active disponible"
              : "Aucune source disponible"}
          </Text>
        ) : (
          <View className="gap-4">
            {visibleSources.length === 0 ? (
              <Text>Aucune source disponible.</Text>
            ) : (
              visibleSources.map((source) => (
                <View
                  key={source.source}
                  className="flex-row flex-wrap justify-between gap-3"
                >
                  <Text>{sourceDisplayLabel(source.source)}</Text>
                  <StatusBadge
                    variant={
                      source.enabled &&
                      ["blocked", "degraded", "stale"].includes(source.status)
                        ? "critical"
                        : "muted"
                    }
                  >
                    {!source.enabled
                      ? "Désactivée"
                      : source.status === "active"
                        ? "Active"
                        : source.status}
                  </StatusBadge>
                </View>
              ))
            )}
          </View>
        )}
      </DialogPanel>
    </View>
  );
}
