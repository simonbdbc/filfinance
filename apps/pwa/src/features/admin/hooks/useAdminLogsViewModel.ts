import { useMemo } from "react";

import type {
  AdminLogDay,
  AdminLogLine,
  ConnectionState,
} from "@shared/api/contracts";
import {
  formatLocalDateTimeWithRaw,
  formatLocalDayKey,
} from "@shared/lib/dateTime";
import { connectionLabel } from "@shared/lib/status";
import type { InspectorField, Metric } from "@shared/ui/workspaceTypes";
import { connectionTone, formatBytes } from "../services/dashboardFormatters";

type UseAdminLogsViewModelOptions = {
  connectionState: ConnectionState;
  days: AdminLogDay[];
  items: AdminLogLine[];
  lastFileSizeBytes: number | null;
  lastHeartbeat: string | null;
  level: string | null;
  query: string;
  selectedDay: string | null;
};

export const adminLogLevelOptions = [
  { label: "Tous les niveaux", value: "" },
  { label: "Info", value: "info" },
  { label: "Avertissement", value: "warn" },
  { label: "Erreur", value: "error" },
  { label: "Débogage", value: "debug" },
  { label: "Trace", value: "trace" },
];

export function useAdminLogsViewModel({
  connectionState,
  days,
  items,
  lastFileSizeBytes,
  lastHeartbeat,
  level,
  query,
  selectedDay,
}: UseAdminLogsViewModelOptions) {
  return useMemo(() => {
    const selectedDayLabel = formatLocalDayKey(selectedDay, "n/a");
    const heartbeatLabel = formatLocalDateTimeWithRaw(lastHeartbeat);

    const metrics: Metric[] = [
      {
        description: `Fichiers disponibles : ${days.length}.`,
        label: "Jour",
        tone: selectedDay ? "info" : "muted",
        value: selectedDayLabel,
      },
      {
        description: `Signal de présence ${heartbeatLabel}`,
        label: "Direct",
        tone: connectionTone(connectionState),
        value: connectionLabel(connectionState),
      },
      {
        description: "Lignes conservées dans la vue actuelle.",
        label: "Vue terminal",
        tone: items.length > 0 ? "trace" : "muted",
        value: items.length,
      },
      {
        description: "Taille observée du fichier sélectionné.",
        label: "Taille",
        tone: lastFileSizeBytes ? "trace" : "muted",
        value: formatBytes(lastFileSizeBytes),
      },
    ];

    const inspectorFields: InspectorField[] = [
      { label: "jour", value: selectedDayLabel },
      { label: "niveau", value: level || "tous" },
      { label: "recherche", value: query.trim() || "aucune" },
      {
        label: "websocket",
        tone: connectionTone(connectionState),
        value: connectionLabel(connectionState),
      },
      { label: "heartbeat", value: heartbeatLabel },
      { label: "taille", value: formatBytes(lastFileSizeBytes) },
    ];

    const dayOptions =
      days.length > 0
        ? days.map((day) => {
            const dayLabel = formatLocalDayKey(day.day, day.day);
            return {
              label: `${dayLabel} - ${formatBytes(day.size_bytes)}`,
              value: day.day,
            };
          })
        : [{ label: "Aucun fichier", value: "" }];

    return {
      dayOptions,
      inspectorFields,
      metrics,
    };
  }, [
    connectionState,
    days,
    items.length,
    lastFileSizeBytes,
    lastHeartbeat,
    level,
    query,
    selectedDay,
  ]);
}
