import { useMemo } from "react";

import type {
  FinancialJuiceBrowserRuntimeStatus,
  FinancialJuiceRuntimeStatus,
  SourceStatus,
  XRuntimeStatus,
} from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import { sourceDisplayLabel } from "@shared/lib/newsSources";
import type {
  HealthItem,
  Metric,
  TimelineItem,
  Tone,
} from "@shared/ui/workspaceTypes";
import {
  activeLabel,
  availableLabel,
  financialJuiceDisabledReasonLabel,
  runtimeTone,
  sourceStatusLabel,
  sourceStatusTone,
  xDisabledReasonLabel,
} from "../services/dashboardFormatters";

type RuntimeControl<Status> = {
  error: string | null;
  isLoading: boolean;
  isUpdating: boolean;
  setEnabled: (enabled: boolean) => void;
  status: Status | null;
};

type BrowserRuntimeControl = {
  error: string | null;
  isLoading: boolean;
  status: FinancialJuiceBrowserRuntimeStatus | null;
};

type UseAdminOverviewViewModelOptions = {
  financialJuiceBrowserRuntime: BrowserRuntimeControl;
  financialJuiceRuntime: RuntimeControl<FinancialJuiceRuntimeStatus>;
  isNewsSourceStatusesLoading: boolean;
  newsSourceStatuses: SourceStatus[];
  newsSourceStatusesError: string | null;
  xRuntime: RuntimeControl<XRuntimeStatus>;
};

export type AdminSourceRuntimeAction = {
  disabled: boolean;
  label: string;
  onPress: () => void;
  variant?: "primary" | "secondary";
};

export type AdminSourceRuntimeRow = {
  actionButtons: AdminSourceRuntimeAction[] | null;
  autoCutLabel: string | null;
  configuredLabel: string;
  diagnostic: string | null;
  enabled: boolean;
  error: string | null;
  label: string;
  serviceLabel: string | null;
  statusLabel: string;
  tone: Tone;
  updatedAtLabel: string;
  switchControl: {
    enabled: boolean;
    isDisabled: boolean;
    isLoading: boolean;
    onValueChange: (enabled: boolean) => void;
  } | null;
};

export function useAdminOverviewViewModel({
  financialJuiceBrowserRuntime,
  financialJuiceRuntime,
  isNewsSourceStatusesLoading,
  newsSourceStatuses,
  newsSourceStatusesError,
  xRuntime,
}: UseAdminOverviewViewModelOptions) {
  return useMemo(() => {
    const sourceTotal = newsSourceStatuses.length;
    const activeSourceTotal = newsSourceStatuses.filter(
      (source) => source.enabled,
    ).length;
    const financialJuiceEnabled = financialJuiceRuntime.status?.enabled;
    const xEnabled = xRuntime.status?.enabled;

    const metrics: Metric[] = [
      {
        description:
          newsSourceStatusesError ??
          (sourceTotal > 0
            ? "Nombre de sources actives parmi les sources répertoriées."
            : "Aucun statut chargé."),
        label: "Sources",
        tone: newsSourceStatusesError
          ? "critical"
          : activeSourceTotal > 0
            ? "info"
            : "muted",
        value: isNewsSourceStatusesLoading
          ? "chargement"
          : newsSourceStatusesError
            ? "restreint"
            : `${activeSourceTotal}/${sourceTotal}`,
      },
      {
        description: financialJuiceRuntime.error ?? "Service de collecte des actualités.",
        label: "FinancialJuice",
        tone: runtimeTone(financialJuiceEnabled, financialJuiceRuntime.error),
        value: activeLabel(financialJuiceEnabled),
      },
      {
        description:
          xRuntime.error ??
          `Persistance ${availableLabel(xRuntime.status?.persistence_available)}`,
        label: "X",
        tone: runtimeTone(xEnabled, xRuntime.error),
        value: activeLabel(xEnabled),
      },
    ];

    const sourceRows: AdminSourceRuntimeRow[] = newsSourceStatuses.map(
      (source) =>
        buildSourceRuntimeRow({
          financialJuiceBrowserRuntime,
          financialJuiceRuntime,
          source,
          xRuntime,
        }),
    );

    const healthItems: HealthItem[] = [
      {
        description: newsSourceStatusesError ?? "Configuration des sources.",
        id: "sources",
        label: "Sources",
        tone: newsSourceStatusesError ? "critical" : "info",
        value: `${activeSourceTotal}/${sourceTotal}`,
      },
      {
        description:
          financialJuiceRuntime.status?.disabled_reason ??
          financialJuiceRuntime.error ??
          "Collecte via le navigateur et interrogation périodique.",
        id: "financial-juice",
        label: "FinancialJuice",
        tone: runtimeTone(financialJuiceEnabled, financialJuiceRuntime.error),
        value: activeLabel(financialJuiceEnabled),
      },
      {
        description:
          xRuntime.status?.disabled_reason ??
          xRuntime.error ??
          "Source sociale et filtres.",
        id: "x",
        label: "X",
        tone: runtimeTone(xEnabled, xRuntime.error),
        value: activeLabel(xEnabled),
      },
      {
        description: "Lecture bornée J/J-1 sans extension de périmètre.",
        id: "logs",
        label: "Journaux",
        tone: "trace",
        value: "prêt",
      },
      {
        description: "Réservé au rôle administrateur.",
        id: "daily-summary",
        label: "Résumé",
        tone: "trace",
        value: "workflow",
      },
    ];

    const timelineItems: TimelineItem[] = [
      ...sourceRows.map((source) => ({
        description: source.diagnostic ?? source.statusLabel,
        id: `source-${source.label}`,
        meta: source.autoCutLabel
          ? `Coupure automatique : ${source.autoCutLabel}`
          : source.configuredLabel,
        title: source.label,
        tone: source.tone,
      })),
      {
        description:
          "Les actions de modification sont disponibles sur les écrans dédiés, selon les permissions et l’état du service.",
        id: "guard-actions",
        meta: "Collecteur navigateur permanent en lecture seule.",
        title: "Protection des opérations",
        tone: "muted",
      },
    ];

    return {
      healthItems,
      metrics,
      sourceRows,
      timelineItems,
    };
  }, [
    financialJuiceBrowserRuntime,
    financialJuiceRuntime,
    isNewsSourceStatusesLoading,
    newsSourceStatuses,
    newsSourceStatusesError,
    xRuntime,
  ]);
}

function buildSourceRuntimeRow({
  financialJuiceBrowserRuntime,
  financialJuiceRuntime,
  source,
  xRuntime,
}: {
  financialJuiceBrowserRuntime: BrowserRuntimeControl;
  financialJuiceRuntime: RuntimeControl<FinancialJuiceRuntimeStatus>;
  source: SourceStatus;
  xRuntime: RuntimeControl<XRuntimeStatus>;
}): AdminSourceRuntimeRow {
  if (source.name === "financial_juice") {
    const enabled = financialJuiceRuntime.status?.enabled ?? source.enabled;
    return {
      actionButtons: null,
      autoCutLabel: financialJuiceRuntime.status?.disabled_reason
        ? financialJuiceDisabledReasonLabel(
            financialJuiceRuntime.status.disabled_reason,
          )
        : null,
      configuredLabel: `fallback env=${activeLabel(
        financialJuiceRuntime.status?.configured_enabled ??
          source.configured_enabled ??
          source.enabled,
      )}`,
      diagnostic: source.message ?? null,
      enabled,
      error: financialJuiceRuntime.error,
      label: sourceDisplayLabel(source.name),
      serviceLabel: null,
      statusLabel: sourceStatusLabel(source.status),
      tone: financialJuiceRuntime.error
        ? "critical"
        : sourceStatusTone(source.status),
      updatedAtLabel: formatLocalDateTimeWithRaw(source.updated_at),
      switchControl: {
        enabled,
        isDisabled:
          financialJuiceRuntime.isLoading || financialJuiceRuntime.isUpdating,
        isLoading: financialJuiceRuntime.isLoading,
        onValueChange: financialJuiceRuntime.setEnabled,
      },
    };
  }

  if (source.name === "x") {
    const enabled = xRuntime.status?.enabled ?? source.enabled;
    return {
      actionButtons: null,
      autoCutLabel: xRuntime.status?.disabled_reason
        ? xDisabledReasonLabel(xRuntime.status.disabled_reason)
        : null,
      configuredLabel: `fallback env=${activeLabel(
        xRuntime.status?.configured_enabled ??
          source.configured_enabled ??
          source.enabled,
      )}`,
      diagnostic: source.message ?? null,
      enabled,
      error: xRuntime.error,
      label: sourceDisplayLabel(source.name),
      serviceLabel: null,
      statusLabel: sourceStatusLabel(source.status),
      tone: xRuntime.error ? "critical" : sourceStatusTone(source.status),
      updatedAtLabel: formatLocalDateTimeWithRaw(source.updated_at),
      switchControl: {
        enabled,
        isDisabled:
          xRuntime.isLoading ||
          xRuntime.isUpdating ||
          !xRuntime.status?.persistence_available ||
          (!xRuntime.status?.bearer_token_configured && !enabled),
        isLoading: xRuntime.isLoading,
        onValueChange: xRuntime.setEnabled,
      },
    };
  }

  if (source.name === "financial_juice_browser") {
    return {
      actionButtons: null,
      autoCutLabel: null,
      configuredLabel: `fallback env=${activeLabel(
        source.configured_enabled ?? source.enabled,
      )}`,
      diagnostic:
        financialJuiceBrowserRuntime.error ??
        financialJuiceBrowserRuntime.status?.effective_message ??
        source.message ??
        null,
      enabled: source.enabled,
      error: financialJuiceBrowserRuntime.error,
      label: sourceDisplayLabel(source.name),
      serviceLabel: `Service navigateur : ${availableLabel(source.service_accessible)} - mainFeed=${availableLabel(source.main_feed_available)}`,
      statusLabel: sourceStatusLabel(source.status),
      tone: financialJuiceBrowserRuntime.error
        ? "critical"
        : sourceStatusTone(source.status),
      updatedAtLabel: formatLocalDateTimeWithRaw(source.updated_at),
      switchControl: null,
    };
  }

  return {
    actionButtons: null,
    autoCutLabel: null,
    configuredLabel: `fallback env=${activeLabel(
      source.configured_enabled ?? source.enabled,
    )}`,
    diagnostic: source.message ?? null,
    enabled: source.enabled,
    error: null,
    label: sourceDisplayLabel(source.name),
    serviceLabel:
      source.name === "financial_juice_browser"
        ? `Service navigateur : ${availableLabel(source.service_accessible)} - mainFeed=${availableLabel(source.main_feed_available)}`
        : null,
    statusLabel: sourceStatusLabel(source.status),
    tone: sourceStatusTone(source.status),
    updatedAtLabel: formatLocalDateTimeWithRaw(source.updated_at),
    switchControl: null,
  };
}
