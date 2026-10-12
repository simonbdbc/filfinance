import { useMemo } from "react";

import type { TranslationGlossaryEntry } from "@shared/api/contracts";
import type { InspectorField, Metric, Tone } from "@shared/ui/workspaceTypes";

type UseAdminGlossaryViewModelOptions = {
  editingId: number | null;
  entries: TranslationGlossaryEntry[];
  formEnabled: boolean;
  formMatchMode: string;
  formPriority: string;
};

export type AdminGlossaryGridRow = TranslationGlossaryEntry & {
  audioLabel: string;
  scopeLabel: string;
  statusTone: Tone;
};

export function useAdminGlossaryViewModel({
  editingId,
  entries,
  formEnabled,
  formMatchMode,
  formPriority,
}: UseAdminGlossaryViewModelOptions) {
  return useMemo(() => {
    const activeCount = entries.filter((entry) => entry.enabled).length;
    const inactiveCount = entries.length - activeCount;
    const audioOverrideCount = entries.filter(
      (entry) => entry.spoken_term,
    ).length;
    const exactCount = entries.filter(
      (entry) => entry.match_mode === "exact",
    ).length;
    const patternCount = entries.filter(
      (entry) => entry.match_mode === "pattern",
    ).length;
    const spokenOnlyCount = entries.filter(
      (entry) => entry.scope === "spoken_only",
    ).length;

    const metrics: Metric[] = [
      {
        description: "Entrées appliquées au pipeline de traduction.",
        label: "Actives",
        tone: activeCount > 0 ? "info" : "muted",
        value: activeCount,
      },
      {
        description: "Entrées actives et inactives conservées dans l’historique.",
        label: "Total",
        tone: entries.length > 0 ? "trace" : "muted",
        value: entries.length,
      },
      {
        description: "Variantes appliquées uniquement au rendu audio.",
        label: "Audio",
        tone: audioOverrideCount > 0 ? "trace" : "muted",
        value: audioOverrideCount,
      },
      {
        description: "Règles qui ne modifient ni la traduction ni l’affichage.",
        label: "Voix uniquement",
        tone: spokenOnlyCount > 0 ? "trace" : "muted",
        value: spokenOnlyCount,
      },
      {
        description: "Règles définies par l’opérateur avec captures <number>.",
        label: "Motifs",
        tone: patternCount > 0 ? "trace" : "muted",
        value: patternCount,
      },
      {
        description: "Mode d’édition courant.",
        label: "Édition",
        tone: editingId ? "trace" : "muted",
        value: editingId ? `#${editingId}` : "nouvelle",
      },
    ];

    const inspectorFields: InspectorField[] = [
      {
        label: "formulaire",
        tone: formEnabled ? "info" : "muted",
        value: formEnabled ? "actif" : "inactif",
      },
      {
        label: "correspondance",
        tone: "trace",
        value: matchModeLabel(formMatchMode),
      },
      { label: "priorité", value: formPriority || "0" },
      {
        label: "exact",
        tone: exactCount > 0 ? "trace" : "muted",
        value: exactCount,
      },
      {
        label: "motifs",
        tone: patternCount > 0 ? "trace" : "muted",
        value: patternCount,
      },
      {
        label: "voix uniquement",
        tone: spokenOnlyCount > 0 ? "trace" : "muted",
        value: spokenOnlyCount,
      },
      {
        label: "inactives",
        tone: inactiveCount > 0 ? "muted" : "info",
        value: inactiveCount,
      },
    ];

    return {
      activeCount,
      inactiveCount,
      inspectorFields,
      metrics,
      rows: entries.map<AdminGlossaryGridRow>((entry) => ({
        ...entry,
        audioLabel: entry.spoken_term ?? "forme affichée",
        scopeLabel: scopeLabel(entry.scope),
        statusTone: entry.enabled ? "info" : "muted",
      })),
    };
  }, [editingId, entries, formEnabled, formMatchMode, formPriority]);
}

function scopeLabel(scope: TranslationGlossaryEntry["scope"]) {
  switch (scope) {
    case "spoken_only":
      return "Voix uniquement";
    case "full_pipeline":
    default:
      return "Pipeline complet";
  }
}

function matchModeLabel(matchMode: string) {
  switch (matchMode) {
    case "exact":
      return "Exact";
    case "pattern":
      return "Motif";
    case "word_boundary":
      return "Mot entier";
    default:
      return matchMode;
  }
}
