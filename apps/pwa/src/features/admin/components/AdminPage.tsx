import { Icon } from "@shared/ui/Icon";
import { Switch } from "@shared/ui/Switch";
import { Text } from "@shared/ui/Text";
import { ActivityIndicator, View, useWindowDimensions } from "react-native";

import type {
  FinancialJuiceBrowserRuntimeStatus,
  FinancialJuiceRuntimeStatus,
  SourceStatus,
  XRuntimeStatus,
} from "@shared/api/contracts";
import { formatLocalDateTime } from "@shared/lib/dateTime";
import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { MetaText, SubtleText } from "@shared/ui/Typography";
import {
  CommandBar,
  DataGrid,
  ErrorState,
  EventTimeline,
  HealthStrip,
  KpiStrip,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
} from "@shared/ui/Workspace";
import {
  type AdminSourceRuntimeRow,
  useAdminOverviewViewModel,
} from "../hooks/useAdminOverviewViewModel";

type AdminPageProps = {
  title: string;
  blacklistLabel: string;
  configSourcesLabel: string;
  dailySummaryLabel: string;
  financialJuiceBrowserRuntime: {
    error: string | null;
    isLoading: boolean;
    status: FinancialJuiceBrowserRuntimeStatus | null;
  };
  financialJuiceRuntime: {
    error: string | null;
    isLoading: boolean;
    isUpdating: boolean;
    setEnabled: (enabled: boolean) => void;
    status: FinancialJuiceRuntimeStatus | null;
  };
  fluxStatusLabel: string;
  glossaryLabel: string;
  historyDebugLabel: string;
  isNewsSourceStatusesLoading: boolean;
  logsLabel: string;
  newsSourceStatuses: SourceStatus[];
  newsSourceStatusesError: string | null;
  onOpenBlacklist: () => void;
  onOpenConfigSources: () => void;
  onOpenDailySummary: () => void;
  onOpenFluxStatus: () => void;
  onOpenGlossary: () => void;
  onOpenHistoryDebug: () => void;
  onOpenLogs: () => void;
  onReloadNewsSourceStatuses: () => void;
  role: string | null;
  xRuntime: {
    error: string | null;
    isLoading: boolean;
    isUpdating: boolean;
    setEnabled: (enabled: boolean) => void;
    status: XRuntimeStatus | null;
  };
};

export function AdminPage({
  title,
  blacklistLabel,
  configSourcesLabel,
  dailySummaryLabel,
  financialJuiceBrowserRuntime,
  financialJuiceRuntime,
  fluxStatusLabel,
  glossaryLabel,
  historyDebugLabel,
  isNewsSourceStatusesLoading,
  logsLabel,
  newsSourceStatuses,
  newsSourceStatusesError,
  onOpenBlacklist,
  onOpenConfigSources,
  onOpenDailySummary,
  onOpenFluxStatus,
  onOpenGlossary,
  onOpenHistoryDebug,
  onOpenLogs,
  onReloadNewsSourceStatuses,
  role,
  xRuntime,
}: AdminPageProps) {
  const { width } = useWindowDimensions();
  const isWideLayout = width >= 1180;
  const viewModel = useAdminOverviewViewModel({
    financialJuiceBrowserRuntime,
    financialJuiceRuntime,
    isNewsSourceStatusesLoading,
    newsSourceStatuses,
    newsSourceStatusesError,
    xRuntime,
  });
  const lastSourceSync = latestSourceSync(newsSourceStatuses);

  return (
    <WorkspacePage
      title={title}
      description="Supervision des sources et des traitements."
      commandBar={
        <CommandBar
          title="Synthèse de la session"
          description="État du service et dernière synchronisation."
          actions={[
            {
              disabled: isNewsSourceStatusesLoading,
              label: "Actualiser",
              onPress: onReloadNewsSourceStatuses,
            },
          ]}
          meta={
            <>
              <StateIndicator tone="muted">rôle {role ?? "n/a"}</StateIndicator>
              <StateIndicator
                tone={lastSourceSync === "n/a" ? "muted" : "trace"}
              >
                sync {lastSourceSync}
              </StateIndicator>
            </>
          }
        />
      }
    >
      <KpiStrip items={viewModel.metrics} />

      <View
        style={{
          flexDirection: isWideLayout ? "row" : "column",
          gap: 24,
          minWidth: 0,
        }}
      >
        <View
          style={{ flex: isWideLayout ? 2 : undefined, minWidth: 0, gap: 24 }}
        >
          <SectionPanel
            title="Statut des sources"
            description="Activation à l’exécution, configuration de repli et diagnostics fournis par l’API d’administration."
            action={
              <Button
                disabled={isNewsSourceStatusesLoading}
                onPress={onReloadNewsSourceStatuses}
              >
                Actualiser
              </Button>
            }
          >
            {isNewsSourceStatusesLoading ? (
              <ActivityIndicator size="small" />
            ) : newsSourceStatusesError ? (
              <ErrorState message={newsSourceStatusesError} />
            ) : (
              <DataGrid
                columns={sourceColumns}
                emptyLabel="Aucun statut de source chargé."
                keyExtractor={(source) => source.label}
                rows={viewModel.sourceRows}
              />
            )}
          </SectionPanel>
          <SectionPanel
            title="Activité récente"
            description="État des opérations et des sources."
          >
            <EventTimeline items={viewModel.timelineItems} />
          </SectionPanel>
        </View>
        <View style={{ flex: isWideLayout ? 1 : undefined, minWidth: 0 }}>
          <SectionPanel
            title="Outils"
            description="Configuration et suivi de votre service."
          >
            <View className="gap-3">
              {[
                { label: configSourcesLabel, onPress: onOpenConfigSources },
                { label: fluxStatusLabel, onPress: onOpenFluxStatus },
                { label: blacklistLabel, onPress: onOpenBlacklist },
                { label: glossaryLabel, onPress: onOpenGlossary },
                { label: historyDebugLabel, onPress: onOpenHistoryDebug },
                { label: dailySummaryLabel, onPress: onOpenDailySummary },
                { label: logsLabel, onPress: onOpenLogs },
              ].map((action) => (
                <Button
                  key={action.label}
                  onPress={action.onPress}
                  suffix={<Icon name="arrow-right" size={18} />}
                >
                  {action.label}
                </Button>
              ))}
            </View>
          </SectionPanel>
        </View>
      </View>
      <SectionPanel
        title="État opérationnel"
        description="Disponibilité des sources et des outils."
      >
        <HealthStrip items={viewModel.healthItems} />
      </SectionPanel>
    </WorkspacePage>
  );
}

const sourceColumns = [
  {
    key: "source",
    label: "Source",
    minWidth: 190,
    render: (source: AdminSourceRuntimeRow) => (
      <SourceNameCell source={source} />
    ),
  },
  {
    key: "runtime",
    label: "Service",
    minWidth: 220,
    render: (source: AdminSourceRuntimeRow) => (
      <SourceRuntimeCell source={source} />
    ),
  },
  {
    key: "diagnostic",
    label: "Diagnostic",
    minWidth: 240,
    render: (source: AdminSourceRuntimeRow) => (
      <View className="gap-1">
        {source.error ? <SourceErrorText message={source.error} /> : null}
        <SubtleText>{source.diagnostic ?? source.statusLabel}</SubtleText>
      </View>
    ),
  },
  {
    key: "action",
    label: "Action",
    minWidth: 150,
    render: (source: AdminSourceRuntimeRow) => (
      <SourceRuntimeToggle source={source} />
    ),
  },
];

function SourceNameCell({ source }: { source: AdminSourceRuntimeRow }) {
  const tokens = useThemeTokens();

  return (
    <View className="gap-2">
      <StateIndicator tone={source.tone}>{source.statusLabel}</StateIndicator>
      <Text
        className="max-w-full min-w-0 break-words text-sm font-extrabold"
        style={{ color: tokens.colors.textPrimary }}
      >
        {source.label}
      </Text>
    </View>
  );
}

function SourceRuntimeCell({ source }: { source: AdminSourceRuntimeRow }) {
  return (
    <View className="gap-1">
      <MetaText>État : {source.enabled ? "actif" : "désactivé"}</MetaText>
      <SubtleText>{source.configuredLabel}</SubtleText>
      <SubtleText>Synchronisation : {source.updatedAtLabel}</SubtleText>
      {source.autoCutLabel ? (
        <SubtleText>Coupure automatique : {source.autoCutLabel}</SubtleText>
      ) : null}
      {source.serviceLabel ? (
        <SubtleText>{source.serviceLabel}</SubtleText>
      ) : null}
    </View>
  );
}

function SourceRuntimeToggle({ source }: { source: AdminSourceRuntimeRow }) {
  const tokens = useThemeTokens();
  const control = source.switchControl;

  if (source.actionButtons) {
    return (
      <View className="gap-2">
        {source.actionButtons.map((action) => (
          <Button
            key={action.label}
            disabled={action.disabled}
            onPress={action.onPress}
            variant={action.variant}
          >
            {action.label}
          </Button>
        ))}
      </View>
    );
  }

  if (!control) {
    return <StateIndicator tone="muted">lecture</StateIndicator>;
  }

  if (control.isLoading) {
    return <ActivityIndicator size="small" color={tokens.colors.textPrimary} />;
  }

  return (
    <Switch
      accessibilityLabel={`Activer ${source.label}`}
      disabled={control.isDisabled}
      onValueChange={control.onValueChange}
      value={control.enabled}
    />
  );
}

function SourceErrorText({ message }: { message: string }) {
  const tokens = useThemeTokens();

  return (
    <MetaText style={{ color: tokens.colors.dangerText }}>{message}</MetaText>
  );
}

function latestSourceSync(sources: SourceStatus[]) {
  const syncDates = sources
    .map((source) => source.updated_at)
    .filter((value): value is string => Boolean(value))
    .sort();
  const latest = syncDates[syncDates.length - 1];

  return latest ? formatLocalDateTime(latest, latest) : "n/a";
}
