import { useMemo } from "react";

import type { XRuntimeStatus } from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import type { InspectorField, Metric } from "@shared/ui/workspaceTypes";
import {
  activeLabel,
  booleanLabel,
  booleanTone,
  formatXAccount,
  runtimeTone,
  xDisabledReasonLabel,
} from "../services/dashboardFormatters";

type UseAdminSourceSettingsViewModelOptions = {
  error: string | null;
  isLoading: boolean;
  isUpdating: boolean;
  status: XRuntimeStatus | null;
};

export function useAdminSourceSettingsViewModel({
  error,
  isLoading,
  isUpdating,
  status,
}: UseAdminSourceSettingsViewModelOptions) {
  return useMemo(() => {
    const accountUsernames = status?.account_usernames ?? [];
    const keywords = status?.keywords ?? [];
    const enabled = status?.enabled ?? false;
    const isMutationDisabled =
      isLoading || isUpdating || !status?.persistence_available;
    const isSwitchDisabled =
      isMutationDisabled || (!status?.bearer_token_configured && !enabled);

    const metrics: Metric[] = [
      {
        description:
          error ?? `Configuration de repli ${activeLabel(status?.configured_enabled)}`,
        label: "Service X",
        tone: runtimeTone(status?.enabled, error),
        value: activeLabel(status?.enabled),
      },
      {
        description: "Comptes suivis par le collecteur X.",
        label: "Comptes",
        tone: accountUsernames.length > 0 ? "info" : "muted",
        value: accountUsernames.length,
      },
      {
        description: "Filtres textuels appliqués à l’ingestion.",
        label: "Mots-clés",
        tone: keywords.length > 0 ? "info" : "muted",
        value: keywords.length,
      },
      {
        description: `Jeton Bearer ${booleanLabel(status?.bearer_token_configured)}`,
        label: "Persistance",
        tone: booleanTone(status?.persistence_available),
        value: booleanLabel(status?.persistence_available),
      },
    ];

    const inspectorFields: InspectorField[] = [
      { label: "id", value: status?.source ?? "default" },
      {
        label: "updated_at",
        value: formatLocalDateTimeWithRaw(status?.updated_at),
      },
      {
        label: "configured_enabled",
        tone: runtimeTone(status?.configured_enabled),
        value: booleanLabel(status?.configured_enabled),
      },
      {
        label: "bearer_token",
        tone: booleanTone(status?.bearer_token_configured),
        value: booleanLabel(status?.bearer_token_configured),
      },
      {
        label: "persistence",
        tone: booleanTone(status?.persistence_available),
        value: booleanLabel(status?.persistence_available),
      },
    ];

    return {
      accountRows: accountUsernames.map((account) => ({
        id: account,
        label: formatXAccount(account),
      })),
      accountUsernames,
      enabled,
      inspectorFields,
      isMutationDisabled,
      isSwitchDisabled,
      keywords,
      keywordRows: keywords.map((keyword) => ({ id: keyword, label: keyword })),
      metrics,
      statusBadges: [
        {
          label: activeLabel(status?.enabled),
          tone: runtimeTone(status?.enabled, error),
        },
        {
          label: `persistance ${booleanLabel(status?.persistence_available)}`,
          tone: booleanTone(status?.persistence_available),
        },
        {
          label: `token ${booleanLabel(status?.bearer_token_configured)}`,
          tone: booleanTone(status?.bearer_token_configured),
        },
      ],
      disabledReason: status?.disabled_reason
        ? xDisabledReasonLabel(status.disabled_reason)
        : null,
    };
  }, [error, isLoading, isUpdating, status]);
}
