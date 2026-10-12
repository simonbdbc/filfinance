import { Text } from "@shared/ui/Text";
import { View } from "react-native";

import type {
  ConnectionState,
  TestRunAccepted,
  TestTraceEvent,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import { connectionLabel, traceStatusLabel } from "@shared/lib/status";
import {
  DataRow,
  SectionPanel,
  StateIndicator,
  type Tone,
} from "@shared/ui/Workspace";

import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { MetaText, SubtleText } from "@shared/ui/Typography";

type TestRunControlCardProps = {
  traceConnectionState: ConnectionState;
  traceWsUrl: string | null;
  isAudioEnabled: boolean;
  activeTestRun: TestRunAccepted | null;
  lastTraceLaggedCount: number | null;
  isTriggeringTestRun: boolean;
  canTriggerTestRun: boolean;
  unavailableReason: string | null;
  lastTraceItem: TestTraceEvent | null;
  testTraceItems: TestTraceEvent[];
  deliveredTestItem: TranslatedNewsItem | null;
  onTriggerTestRun: () => void;
};

export function TestRunControlCard({
  traceConnectionState,
  traceWsUrl,
  isAudioEnabled,
  activeTestRun,
  lastTraceLaggedCount,
  isTriggeringTestRun,
  canTriggerTestRun,
  unavailableReason,
  lastTraceItem,
  testTraceItems,
  deliveredTestItem,
  onTriggerTestRun,
}: TestRunControlCardProps) {
  const tokens = useThemeTokens();
  const phaseItems = buildTestRunPhaseItems(
    activeTestRun,
    testTraceItems,
    deliveredTestItem,
  );
  const textLivePhase = phaseItems.find((phase) => phase.id === "text_live");
  const telegramPhase = phaseItems.find((phase) => phase.id === "telegram");
  const audioPhase = phaseItems.find((phase) => phase.id === "audio_ready");
  const cardDescription =
    "Le bouton envoie un signal via `/ws/test-runs`, injecte une actualité synthétique dans le " +
    "pipeline canonique, puis affiche chaque étape de l’exécution jusqu’à Telegram, `/ws/news` " +
    "et `audio_ready` HLS.";

  return (
    <SectionPanel
      title="Test de bout en bout"
      description={cardDescription}
      action={
        <StateIndicator
          tone={traceConnectionState === "open" ? "trace" : "muted"}
        >
          {traceConnectionState === "open" ? "TRACE" : "DEBUG"}
        </StateIndicator>
      }
    >
      <SubtleText>Canal de débogage : {traceWsUrl}</SubtleText>
      <MetaText>
        État du débogage : {connectionLabel(traceConnectionState)}
        {activeTestRun ? ` • run_id=${activeTestRun.run_id}` : ""}
        {activeTestRun
          ? ` • début=${formatLocalDateTimeWithRaw(activeTestRun.accepted_at)}`
          : ""}
        {lastTraceLaggedCount ? ` • lag=${lastTraceLaggedCount}` : ""}
      </MetaText>

      <View className="flex-row flex-wrap gap-2">
        <StateIndicator tone={textLivePhase?.tone ?? "muted"}>
          Texte en direct {textLivePhase?.state === "done" ? "OK" : "..."}
        </StateIndicator>
        <StateIndicator tone={telegramPhase?.tone ?? "muted"}>
          Telegram {telegramPhase?.state === "done" ? "OK" : "..."}
        </StateIndicator>
        <StateIndicator tone={audioPhase?.tone ?? "muted"}>
          audio_ready {audioPhase?.state === "done" ? "OK" : "..."}
        </StateIndicator>
      </View>

      {phaseItems.length > 0 ? (
        <View className="gap-2">
          {phaseItems.map((phase) => (
            <DataRow key={phase.id}>
              <View className="flex-row flex-wrap items-center justify-between gap-2">
                <Text
                  className="text-sm font-extrabold uppercase tracking-[0.8px]"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {phase.label}
                </Text>
                <StateIndicator tone={phase.tone}>{phase.badge}</StateIndicator>
              </View>
              <Text
                className="text-sm leading-[18px]"
                style={{ color: tokens.colors.textPrimary }}
              >
                {phase.detail}
              </Text>
            </DataRow>
          ))}
        </View>
      ) : null}

      {!isAudioEnabled ? (
        <DataRow
          style={{
            backgroundColor: tokens.colors.surface,
            borderColor: tokens.colors.borderSoft,
          }}
        >
          <SubtleText
            className="text-sm font-semibold leading-5"
            style={{ color: tokens.colors.textPrimary }}
          >
            Le direct audio est désactivé. Le test validera le pipeline serveur et
            `audio_ready`, mais aucun son ne sera lu sur ce client.
          </SubtleText>
        </DataRow>
      ) : null}

      <Button
        variant="primary"
        disabled={!canTriggerTestRun || isTriggeringTestRun}
        textClassName="text-center"
        onPress={onTriggerTestRun}
      >
        {!canTriggerTestRun
          ? "Test réservé aux administrateurs"
          : isTriggeringTestRun
            ? "Déclenchement…"
            : traceConnectionState === "open"
              ? "Déclencher un test Telegram + WS"
              : "Reconnecter TRACE puis déclencher le test"}
      </Button>

      {!canTriggerTestRun && unavailableReason ? (
        <SubtleText>{unavailableReason}</SubtleText>
      ) : null}

      {lastTraceItem ? (
        <DataRow>
          <Text
            className="text-sm font-bold uppercase tracking-[0.8px]"
            style={{ color: tokens.colors.textPrimary }}
          >
            Dernière étape
          </Text>
          <Text
            className="text-sm font-extrabold uppercase tracking-[0.8px]"
            style={{ color: tokens.colors.textPrimary }}
          >
            {lastTraceItem.stage}
          </Text>
          <Text
            className="text-sm leading-[19px]"
            style={{ color: tokens.colors.textPrimary }}
          >
            {traceStatusLabel(lastTraceItem.status)} •{" "}
            {formatLocalDateTimeWithRaw(lastTraceItem.timestamp)}
          </Text>
          <Text
            className="text-sm leading-5"
            style={{ color: tokens.colors.textSecondary }}
          >
            {lastTraceItem.message}
          </Text>
          {deliveredTestItem ? (
            <Text
              className="text-sm font-semibold leading-[19px]"
              style={{ color: tokens.colors.successText }}
            >
              Élément reçu par le client : {deliveredTestItem.source_title}
              {" -> "}
              {deliveredTestItem.translated_title}
            </Text>
          ) : null}
        </DataRow>
      ) : (
        <SubtleText>
          Aucun test actif. Déclenchez un test pour suivre l’injection,
          la traduction, Telegram, le flux texte et le clip HLS audio_ready.
        </SubtleText>
      )}
    </SectionPanel>
  );
}

type TestRunPhaseState = "done" | "failed" | "pending" | "running";

type TestRunPhaseItem = {
  badge: string;
  detail: string;
  id: string;
  label: string;
  state: TestRunPhaseState;
  tone: Tone;
};

type TestRunPhaseDefinition = {
  id: string;
  label: string;
  pendingLabel: string;
  progressStages: string[];
  successStages: string[];
  thresholdMs: number;
};

const TEST_RUN_PHASES: TestRunPhaseDefinition[] = [
  {
    id: "ingestion",
    label: "Ingestion",
    pendingLabel: "attente pipeline",
    progressStages: ["test_run_enqueued", "pipeline_received"],
    successStages: ["pipeline_persisted"],
    thresholdMs: 3_000,
  },
  {
    id: "translation",
    label: "Traduction",
    pendingLabel: "attente traduction",
    progressStages: ["translation_received"],
    successStages: ["translation_persisted", "translation_broadcast_emitted"],
    thresholdMs: 10_000,
  },
  {
    id: "text_live",
    label: "Texte en direct",
    pendingLabel: "attente du WebSocket client",
    progressStages: ["translation_broadcast_emitted"],
    successStages: ["user_ws_emitted", "user_ws_replay_emitted"],
    thresholdMs: 3_000,
  },
  {
    id: "telegram",
    label: "Telegram",
    pendingLabel: "attente Telegram",
    progressStages: ["telegram_queue_enqueued", "telegram_send_started"],
    successStages: ["telegram_send_succeeded"],
    thresholdMs: 10_000,
  },
  {
    id: "audio_ready",
    label: "Audio HLS",
    pendingLabel: "attente audio_ready",
    progressStages: ["tts_received", "tts_clip_body_generated"],
    successStages: ["audio_ready_published"],
    thresholdMs: 20_000,
  },
];

function buildTestRunPhaseItems(
  activeTestRun: TestRunAccepted | null,
  traceItems: TestTraceEvent[],
  deliveredTestItem: TranslatedNewsItem | null,
): TestRunPhaseItem[] {
  if (!activeTestRun) {
    return [];
  }

  const acceptedAtMs = Date.parse(activeTestRun.accepted_at);
  const acceptedPhase: TestRunPhaseItem = {
    badge: "OK",
    detail: `depart ${formatLocalDateTimeWithRaw(activeTestRun.accepted_at)}`,
    id: "accepted",
    label: "Acceptation",
    state: "done",
    tone: "info",
  };

  return [
    acceptedPhase,
    ...TEST_RUN_PHASES.map((phase) =>
      buildPhaseItem(phase, traceItems, deliveredTestItem, acceptedAtMs),
    ),
  ];
}

function buildPhaseItem(
  phase: TestRunPhaseDefinition,
  traceItems: TestTraceEvent[],
  deliveredTestItem: TranslatedNewsItem | null,
  acceptedAtMs: number,
): TestRunPhaseItem {
  const failed = latestTraceForStages(
    traceItems,
    failureStagesForPhase(phase.id),
  );
  if (failed) {
    return {
      badge: "erreur",
      detail: `${failed.stage} • ${failed.message}`,
      id: phase.id,
      label: phase.label,
      state: "failed",
      tone: "critical",
    };
  }

  const success =
    phase.id === "text_live" && deliveredTestItem
      ? syntheticDeliveredTrace(deliveredTestItem)
      : latestTraceForStages(traceItems, phase.successStages);
  if (success) {
    const durationMs = phaseDurationMs(acceptedAtMs, success.timestamp);
    const slow = durationMs !== null && durationMs > phase.thresholdMs;
    return {
      badge: slow ? "lent" : "OK",
      detail: `${success.stage} • ${durationLabel(durationMs)} depuis acceptation`,
      id: phase.id,
      label: phase.label,
      state: "done",
      tone: slow ? "trace" : "info",
    };
  }

  const progress = latestTraceForStages(traceItems, phase.progressStages);
  if (progress) {
    const durationMs = phaseDurationMs(acceptedAtMs, progress.timestamp);
    return {
      badge: "en cours",
      detail: `${progress.stage} • ${durationLabel(durationMs)} depuis acceptation`,
      id: phase.id,
      label: phase.label,
      state: "running",
      tone: "trace",
    };
  }

  return {
    badge: "...",
    detail: phase.pendingLabel,
    id: phase.id,
    label: phase.label,
    state: "pending",
    tone: "muted",
  };
}

function latestTraceForStages(
  traceItems: TestTraceEvent[],
  stages: string[],
): TestTraceEvent | null {
  const stageSet = new Set(stages);
  for (let index = traceItems.length - 1; index >= 0; index -= 1) {
    const item = traceItems[index];
    if (stageSet.has(item.stage)) {
      return item;
    }
  }
  return null;
}

function failureStagesForPhase(phaseId: string) {
  if (phaseId === "text_live") {
    return ["user_ws_send_failed", "user_ws_serialization_failed"];
  }
  if (phaseId === "telegram") {
    return ["telegram_queue_dropped", "telegram_send_failed"];
  }
  if (phaseId === "audio_ready") {
    return ["tts_failed"];
  }
  if (phaseId === "translation") {
    return ["translation_persist_failed"];
  }
  return ["pipeline_persist_failed", "pipeline_duplicate_dropped"];
}

function syntheticDeliveredTrace(
  deliveredTestItem: TranslatedNewsItem,
): TestTraceEvent {
  return {
    detail: null,
    message: "Élément reçu par le client.",
    run_id: deliveredTestItem.test_run_id ?? "",
    stage: "client_live_item_received",
    status: "ok",
    timestamp:
      deliveredTestItem.timestamp ??
      deliveredTestItem.updated_at ??
      deliveredTestItem.created_at,
  };
}

function phaseDurationMs(acceptedAtMs: number, timestamp: string) {
  const timestampMs = Date.parse(timestamp);
  if (!Number.isFinite(acceptedAtMs) || !Number.isFinite(timestampMs)) {
    return null;
  }
  return Math.max(0, timestampMs - acceptedAtMs);
}

function durationLabel(durationMs: number | null) {
  if (durationMs === null) {
    return "durée n/a";
  }
  if (durationMs < 1_000) {
    return `${durationMs} ms`;
  }
  if (durationMs < 60_000) {
    return `${(durationMs / 1_000).toFixed(1)} s`;
  }
  const minutes = Math.floor(durationMs / 60_000);
  const seconds = Math.round((durationMs % 60_000) / 1_000);
  return `${minutes} min ${seconds} s`;
}
