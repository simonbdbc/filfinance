import { useMemo } from "react";

import type {
  AdminCalendarDayToday,
  CalendarDayRun,
  ConnectionState,
  CalendarDayRunStep,
} from "@shared/api/contracts";
import {
  formatLocalDateTimeWithRaw,
  formatLocalDayKey,
} from "@shared/lib/dateTime";
import { connectionLabel } from "@shared/lib/status";
import type {
  Tone,
  HealthItem,
  InspectorField,
  Metric,
  TimelineItem,
  WorkflowStage,
} from "@shared/ui/workspaceTypes";
import {
  calendarSummaryDisplayStatus,
  calendarSummaryStatusTone,
  connectionTone,
  hasCalendarSummaryAudioPlaylist,
  runStepTone,
} from "../services/dashboardFormatters";

type UseAdminDailySummaryViewModelOptions = {
  connectionState: ConnectionState;
  lastHeartbeat: string | null;
  snapshot: AdminCalendarDayToday | null;
};

const workflowStageLabels = [
  { id: "collecte", title: "Collecte" },
  { id: "qualification", title: "Qualification" },
  { id: "synthese", title: "Synthèse IA" },
  { id: "audio", title: "Génération audio" },
  { id: "publication", title: "Publication" },
] as const;

type WorkflowStageId = (typeof workflowStageLabels)[number]["id"];

type WorkflowStepView = Pick<
  CalendarDayRunStep,
  "createdAt" | "detail" | "message" | "stage" | "status"
> & {
  id: string;
};

export function useAdminDailySummaryViewModel({
  connectionState,
  lastHeartbeat,
  snapshot,
}: UseAdminDailySummaryViewModelOptions) {
  return useMemo(() => {
    const summary = snapshot?.summary;
    const summaryDisplayStatus = calendarSummaryDisplayStatus(summary);
    const summaryPlaylistUrl =
      summary?.currentPlaylistUrl ??
      summary?.playlistUrl ??
      summary?.immutablePlaylistUrl ??
      null;
    const run = snapshot?.latestRun;
    const localDayLabel = formatLocalDayKey(snapshot?.localDay, "n/a");
    const heartbeatLabel = formatLocalDateTimeWithRaw(lastHeartbeat);
    const itemCount = snapshot?.items.length ?? 0;
    const highItemCount =
      snapshot?.items.filter((item) => item.finalImportance === "high")
        .length ?? 0;
    const windowLabel =
      snapshot?.automaticWindowStart && snapshot?.automaticWindowEnd
        ? `${snapshot.automaticWindowStart} -> ${snapshot.automaticWindowEnd}`
        : "01:30 -> 03:00";
    const metrics: Metric[] = [
      {
        description: `Jour ${localDayLabel}`,
        label: "Calendrier",
        tone: calendarSummaryStatusTone(summary),
        value: snapshot?.state.calendarIngestionStatus ?? "missing",
      },
      {
        description: `Signal de présence ${heartbeatLabel}`,
        label: "WebSocket",
        tone: connectionTone(connectionState),
        value: connectionLabel(connectionState),
      },
      {
        description: `Nouvelle tentative automatique : ${Math.round((snapshot?.automaticRetryBaseSecs ?? 1800) / 60)} min.`,
        label: "Plage automatique",
        tone:
          snapshot?.automaticWindowStatus === "inside_window"
            ? "info"
            : "muted",
        value:
          snapshot?.automaticWindowStatus === "manual_only"
            ? "manuel"
            : windowLabel,
      },
      {
        description: "Événements fusionnés dans la base du calendrier.",
        label: "Événements",
        tone: itemCount > 0 ? "info" : "muted",
        value: itemCount,
      },
      {
        description: "Événements d’importance élevée avec audio ou alertes.",
        label: "Importance élevée",
        tone: highItemCount > 0 ? "trace" : "muted",
        value: highItemCount,
      },
    ];

    const healthItems: HealthItem[] = [
      {
        description: run?.reason ?? "Aucune exécution récente.",
        id: "run",
        label: "Exécution",
        tone: runStepTone(run?.status),
        value: run?.status ?? "n/a",
      },
      {
        description: summaryPlaylistUrl ?? "Liste de lecture non publiée.",
        id: "playlist",
        label: "Liste de lecture",
        tone: hasCalendarSummaryAudioPlaylist(summary) ? "info" : "muted",
        value: hasCalendarSummaryAudioPlaylist(summary) ? "prête" : "n/a",
      },
      {
        description: `Artefacts collectés : ${snapshot?.artifacts.length ?? 0}.`,
        id: "artifacts",
        label: "Artefacts",
        tone: (snapshot?.artifacts.length ?? 0) > 0 ? "trace" : "muted",
        value: snapshot?.artifacts.length ?? 0,
      },
    ];

    const inspectorFields: InspectorField[] = [
      { label: "jour", value: localDayLabel },
      { label: "summary_id", value: summary?.id ?? "n/a" },
      { label: "run_id", value: run?.runId ?? "n/a" },
      {
        label: "run_actif",
        value: snapshot?.activePendingRunId ?? "aucun",
      },
      { label: "raison", value: run?.reason ?? "n/a" },
      { label: "workflow", value: run?.workflowKind ?? "n/a" },
      { label: "moteur", value: calendarRunEngineLabel(run) },
      { label: "items_high_bdd", value: highItemCount },
      { label: "last_error", value: snapshot?.state.lastError ?? "n/a" },
      {
        label: "fenetre_auto",
        value: `${windowLabel} (${snapshot?.automaticWindowStatus ?? "n/a"})`,
      },
      {
        label: "heartbeat",
        tone: connectionTone(connectionState),
        value: heartbeatLabel,
      },
    ];

    const workflowStages = buildWorkflowStages(
      snapshot?.steps ?? [],
      summaryDisplayStatus,
      run?.workflowKind,
    );
    const timelineItems: TimelineItem[] = (snapshot?.steps ?? [])
      .slice(-8)
      .reverse()
      .map((step) => ({
        description: step.detail ?? step.message,
        id: String(step.id),
        meta: formatLocalDateTimeWithRaw(step.createdAt),
        title: step.stage,
        tone: runStepTone(step.status),
      }));

    return {
      healthItems,
      inspectorFields,
      metrics,
      timelineItems,
      workflowStages,
    };
  }, [connectionState, lastHeartbeat, snapshot]);
}

function buildWorkflowStages(
  steps: CalendarDayRunStep[],
  summaryStatus: string | null | undefined,
  workflowKind: string | null | undefined,
): WorkflowStage[] {
  const normalizedSteps = normalizeWorkflowSteps(steps);
  return workflowStageLabels.map((stage) => {
    const stageSteps = normalizedSteps.filter(
      (step) => stageIdForStep(step, workflowKind) === stage.id,
    );
    return {
      description:
        stageSteps.length > 0
          ? `Étapes : ${stageSteps.length}`
          : "En attente d’un signal d’exécution.",
      id: stage.id,
      items:
        stageSteps.length > 0
          ? stageSteps.map((step) => ({
              description: step.detail ?? step.message,
              id: step.id,
              meta: `${step.status} - ${formatLocalDateTimeWithRaw(step.createdAt)}`,
              title: step.stage,
              tone: runStepTone(step.status),
            }))
          : [
              {
                description:
                  summaryStatus === "audio_ready"
                    ? "Aucune étape détaillée reçue dans cet instantané."
                    : "Étape non déclenchée lors de la dernière exécution.",
                id: `${stage.id}-empty`,
                meta: summaryStatus ?? "missing",
                title: "Attente",
                tone: "muted" as const,
              },
            ],
      title: stage.title,
      tone: workflowStageTone(stageSteps),
    };
  });
}

function normalizeWorkflowSteps(
  steps: CalendarDayRunStep[],
): WorkflowStepView[] {
  const seen = new Set<string>();
  const deduped: WorkflowStepView[] = [];

  for (const step of steps) {
    const key = workflowStepDedupeKey(step);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    deduped.push({
      createdAt: step.createdAt,
      detail: step.detail,
      id: String(step.id),
      message: step.message,
      stage: step.stage,
      status: step.status,
    });
  }

  return mergeLegacyWebhookTransitions(deduped);
}

function workflowStepDedupeKey(step: CalendarDayRunStep) {
  return [
    step.stage,
    step.status,
    step.message,
    normalizeWorkflowDetailForDedupe(step.detail),
  ].join("|");
}

function normalizeWorkflowDetailForDedupe(detail: string | null) {
  return (detail ?? "")
    .replace(
      /\b(?:duration_ms|input_count|output_count|high_count)=\d+\b/gu,
      "",
    )
    .replace(/\s+/gu, " ")
    .trim();
}

function mergeLegacyWebhookTransitions(
  steps: WorkflowStepView[],
): WorkflowStepView[] {
  const merged: WorkflowStepView[] = [];
  const webhookIndexes = new Map<
    string,
    { accepted?: WorkflowStepView; index: number; sent?: WorkflowStepView }
  >();

  for (const step of steps) {
    const match = /^n8n_calendar_(.+)_webhook_(sent|accepted)$/u.exec(
      step.stage,
    );
    if (!match) {
      merged.push(step);
      continue;
    }

    const key = `n8n_calendar_${match[1]}_webhook`;
    const kind = match[2] as "accepted" | "sent";
    const current = webhookIndexes.get(key) ?? { index: merged.length };
    current[kind] = step;
    webhookIndexes.set(key, current);
    merged[current.index] = mergedLegacyWebhookStep(
      key,
      current.sent,
      current.accepted,
    );
  }

  return merged;
}

function mergedLegacyWebhookStep(
  stage: string,
  sent: WorkflowStepView | undefined,
  accepted: WorkflowStepView | undefined,
): WorkflowStepView {
  const latest = accepted ?? sent;
  if (!latest) {
    return {
      createdAt: "",
      detail: null,
      id: stage,
      message: "Transition webhook historique du calendrier.",
      stage,
      status: "info",
    };
  }

  return {
    ...latest,
    detail: mergedLegacyWebhookDetail(sent, accepted),
    id: `${stage}-${sent?.id ?? "missing"}-${accepted?.id ?? "pending"}`,
    message: accepted?.message ?? sent?.message ?? latest.message,
    stage,
    status: accepted?.status ?? sent?.status ?? latest.status,
  };
}

function mergedLegacyWebhookDetail(
  sent: WorkflowStepView | undefined,
  accepted: WorkflowStepView | undefined,
) {
  const details = [];
  if (sent?.detail) {
    details.push(`sent: ${sent.detail}`);
  }
  if (accepted?.detail && accepted.detail !== sent?.detail) {
    details.push(`accepted: ${accepted.detail}`);
  }
  return details.length > 0 ? details.join(" | ") : null;
}

function calendarRunEngineLabel(run: CalendarDayRun | null | undefined) {
  if (run?.n8nWorkflowId || run?.n8nWorkflowName || run?.n8nExecutionId) {
    return "n8n (historique)";
  }
  return "Rust natif";
}

function workflowStageTone(steps: WorkflowStepView[]): Tone {
  const tones = steps.map((step) => runStepTone(step.status));
  if (tones.includes("critical")) {
    return "critical";
  }
  if (tones.includes("info")) {
    return "info";
  }
  if (tones.includes("trace")) {
    return "trace";
  }
  return "muted";
}

function stageIdForStep(
  step: WorkflowStepView,
  fallbackWorkflowKind: string | null | undefined,
): WorkflowStageId {
  const stage = step.stage.toLowerCase();
  const message = step.message.toLowerCase();
  const detail = (step.detail ?? "").toLowerCase();
  const haystack = `${stage} ${message} ${detail}`;
  const workflowKind = workflowKindForStep(step, fallbackWorkflowKind);

  if (isAudioStep(haystack)) {
    return "audio";
  }
  if (isPublicationStep(haystack)) {
    return "publication";
  }

  if (workflowKind === "summary") {
    if (isQualificationStep(haystack) || haystack.includes("items_fetch")) {
      return "qualification";
    }
    return "synthese";
  }

  if (isQualificationStep(haystack)) {
    return "qualification";
  }
  if (isCollectionStep(haystack)) {
    return "collecte";
  }

  return "synthese";
}

function workflowKindForStep(
  step: WorkflowStepView,
  fallbackWorkflowKind: string | null | undefined,
) {
  const match = /\bworkflow_kind=([a-z_]+)/u.exec(step.detail ?? "");
  return match?.[1] ?? fallbackWorkflowKind ?? null;
}

function isCollectionStep(haystack: string) {
  return (
    haystack.includes("collect") ||
    haystack.includes("load") ||
    haystack.includes("source") ||
    haystack.includes("news") ||
    haystack.includes("fetch") ||
    haystack.includes("ingest")
  );
}

function isQualificationStep(haystack: string) {
  return (
    haystack.includes("qualif") ||
    haystack.includes("important") ||
    haystack.includes("rank") ||
    haystack.includes("normalis") ||
    haystack.includes("prompt")
  );
}

function isAudioStep(haystack: string) {
  return (
    haystack.includes("audio") ||
    haystack.includes("tts") ||
    haystack.includes("narration") ||
    haystack.includes("hls")
  );
}

function isPublicationStep(haystack: string) {
  return (
    haystack.includes("publish") ||
    haystack.includes("publication") ||
    haystack.includes("playlist") ||
    haystack.includes("persist") ||
    haystack.includes("final") ||
    haystack.includes("callback")
  );
}
