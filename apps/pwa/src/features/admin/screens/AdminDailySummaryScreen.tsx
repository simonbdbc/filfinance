import { Text } from "@shared/ui/Text";
import { useMemo, useState } from "react";
import { ScrollView, View, useWindowDimensions } from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type {
  AdminCalendarDayToday,
  CalendarDayItem,
  CalendarDayItemAudio,
  CalendarDayRun,
  CalendarDayRunArtifact,
} from "@shared/api/contracts";
import {
  businessWeekDaysForLocalDay,
  defaultBusinessCalendarDayKey,
  formatLocalDateTimeWithRaw,
  shortWeekdayLabel,
} from "@shared/lib/dateTime";
import { connectionLabel } from "@shared/lib/status";
import { useThemeTokens } from "@shared/theme/theme";
import {
  AccessDeniedCard,
  Button,
  MetaText,
  SubtleText,
  UserProfileLoadingCard,
} from "@shared/ui";
import {
  CommandBar,
  ErrorState,
  EventTimeline,
  Grid,
  HealthStrip,
  InspectorPanel,
  KpiStrip,
  DataRow,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
  TriPane,
  WorkflowBoard,
} from "@shared/ui/Workspace";
import { useAdminDailySummaryLive } from "../hooks/useAdminDailySummaryLive";
import { useAdminDailySummaryViewModel } from "../hooks/useAdminDailySummaryViewModel";
import {
  calendarSummaryDisplayStatus,
  calendarSummaryStatusTone,
  connectionTone,
  formatBytes,
  formatArtifactContent,
  runStepTone,
} from "../services/dashboardFormatters";
import type { Tone } from "@shared/ui/workspaceTypes";

import { CalendarLogsPanel } from "../components/CalendarLogsPanel";

export function AdminDailySummaryScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const { width } = useWindowDimensions();
  const isWideLayout = width >= 1080;
  const [selectedDay, setSelectedDay] = useState(defaultBusinessCalendarDayKey);
  const businessDays = useMemo(
    () => businessWeekDaysForLocalDay(selectedDay),
    [selectedDay],
  );
  const runtime = useAdminDailySummaryLive({
    authSession,
    isEnabled: canAccessAdmin && role === "admin",
    localDay: selectedDay,
    wsUrl: server.wsUrl,
  });
  const snapshot = runtime.snapshot;
  const summary = snapshot?.summary;
  const summaryDisplayStatus = calendarSummaryDisplayStatus(summary);
  const hasPendingRun = snapshot?.hasActivePendingRun ?? false;
  const viewModel = useAdminDailySummaryViewModel({
    connectionState: runtime.connectionState,
    lastHeartbeat: runtime.lastHeartbeat,
    snapshot,
  });

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin || role !== "admin") {
    return <AccessDeniedCard role={role} />;
  }

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminDailySummary}
      description="Pilotage et suivi en temps réel du calendrier économique du jour."
      commandBar={
        <CommandBar
          title="Traitement quotidien du calendrier"
          description="Collecte des calendriers, fusion, qualification, résumé, audio et alertes."
          actions={[
            {
              disabled: runtime.isLoading,
              label: "Actualiser",
              onPress: runtime.reload,
            },
            {
              disabled: runtime.isRunning || hasPendingRun,
              label: hasPendingRun
                ? "Traitement déjà en cours"
                : runtime.isRunning
                  ? "Relance..."
                  : "Relancer le jour",
              onPress: runtime.run,
              variant: "primary",
            },
            {
              disabled: runtime.isRunning,
              label: runtime.isRunning ? "Relance..." : "Relancer la semaine",
              onPress: runtime.runWeek,
            },
            {
              label: "Reconnecter",
              onPress: runtime.reconnect,
            },
          ]}
          meta={
            <>
              <StateIndicator tone={calendarSummaryStatusTone(summary)}>
                {summaryDisplayStatus}
              </StateIndicator>
              <StateIndicator tone={connectionTone(runtime.connectionState)}>
                {connectionLabel(runtime.connectionState)}
              </StateIndicator>
            </>
          }
        />
      }
    >
      <BusinessDaySelector
        days={businessDays}
        selectedDay={selectedDay}
        onSelectDay={setSelectedDay}
      />
      <KpiStrip items={viewModel.metrics} />
      {runtime.error ? <ErrorState message={runtime.error} /> : null}
      <InconsistenciesPanel snapshot={snapshot} />

      <SectionPanel
        title="Traitement du jour"
        description="Vue par colonnes de l’exécution d’administration en cours."
      >
        <WorkflowBoard stages={viewModel.workflowStages} />
      </SectionPanel>

      <CalendarLogsPanel authSession={authSession} wsUrl={server.wsUrl} />

      <TriPane
        isWide={isWideLayout}
        railWidth={300}
        inspectorWidth={360}
        rail={
          <SectionPanel
            title="Journal d’activité"
            description="Derniers signaux reçus pendant l’exécution."
          >
            <EventTimeline
              emptyLabel="Aucune étape reçue."
              items={viewModel.timelineItems}
            />
          </SectionPanel>
        }
        main={
          <Grid direction="vertical">
            <Grid direction="horizontal" minItemWidth={320}>
              <CalendarRunsPanel snapshot={snapshot} />
              <ArtifactsPanel artifacts={snapshot?.artifacts ?? []} />
              <FinalTextsPanel snapshot={snapshot} />
            </Grid>
            <NewsUsedPanel snapshot={snapshot} />
          </Grid>
        }
        inspector={
          <InspectorPanel
            title="Détails de l’exécution"
            description="État du résumé actuel, de la dernière exécution et de la connexion en direct."
            fields={viewModel.inspectorFields}
          >
            <HealthStrip items={viewModel.healthItems} />
            {summary?.playlistUrl ? (
              <SubtleText>Liste de lecture : {summary.playlistUrl}</SubtleText>
            ) : null}
            {summary?.currentPlaylistUrl ? (
              <SubtleText>Actuelle : {summary.currentPlaylistUrl}</SubtleText>
            ) : null}
            {summary?.immutablePlaylistUrl ? (
              <SubtleText>Immuable : {summary.immutablePlaylistUrl}</SubtleText>
            ) : null}
            {summary?.errorMessage || snapshot?.latestRun?.errorMessage ? (
              <ErrorState
                message={
                  summary?.errorMessage ||
                  snapshot?.latestRun?.errorMessage ||
                  ""
                }
              />
            ) : null}
          </InspectorPanel>
        }
      />
    </WorkspacePage>
  );
}

function BusinessDaySelector({
  days,
  onSelectDay,
  selectedDay,
}: {
  days: string[];
  onSelectDay: (day: string) => void;
  selectedDay: string;
}) {
  return (
    <SectionPanel
      title="Semaine ouvrée"
      description="Sélection du jour du calendrier suivi par l’administrateur."
    >
      <View className="flex-row flex-wrap items-center gap-2">
        {days.map((day) => (
          <Button
            className="min-w-[112px] flex-1"
            key={day}
            onPress={() => onSelectDay(day)}
            variant={day === selectedDay ? "primary" : "secondary"}
          >
            {shortWeekdayLabel(day)}
          </Button>
        ))}
      </View>
    </SectionPanel>
  );
}

function CalendarRunsPanel({
  snapshot,
}: {
  snapshot: AdminCalendarDayToday | null;
}) {
  const [isHistoryVisible, setIsHistoryVisible] = useState(false);
  const runs = (snapshot?.runs ?? [])
    .filter(
      (run) =>
        run.workflowKind === "ingestion" || run.workflowKind === "summary",
    )
    .slice()
    .sort((left, right) => right.id - left.id);
  const currentRun =
    runs.find((run) => run.runId === snapshot?.activePendingRunId) ??
    snapshot?.latestRun ??
    runs[0] ??
    null;
  const historyRuns = runs
    .filter((run) => run.runId !== currentRun?.runId)
    .slice(0, 8);
  const ingestionRuns = historyRuns.filter(
    (run) => run.workflowKind === "ingestion",
  );
  const summaryRuns = historyRuns.filter(
    (run) => run.workflowKind === "summary",
  );

  return (
    <>
      <CurrentWorkflowRunPanel run={currentRun} />
      <SectionPanel
        title="Historique des exécutions du calendrier"
        description="Exécutions précédentes du jour, séparées de l’exécution en cours."
      >
        <View className="gap-3">
          <View className="flex-row flex-wrap items-center gap-2">
            <StateIndicator tone="muted">
              Exécutions antérieures : {historyRuns.length}
            </StateIndicator>
            <Button
              className="self-start"
              onPress={() => setIsHistoryVisible((current) => !current)}
            >
              {isHistoryVisible
                ? "Masquer l’historique"
                : "Afficher l’historique"}
            </Button>
          </View>
          {isHistoryVisible ? (
            <Grid direction="horizontal" minItemWidth={260}>
              <WorkflowRunColumn title="Ingestion" runs={ingestionRuns} />
              <WorkflowRunColumn title="Résumé" runs={summaryRuns} />
            </Grid>
          ) : (
            <SubtleText>
              Historique masqué par défaut pour garder le focus sur le run
              courant.
            </SubtleText>
          )}
        </View>
      </SectionPanel>
    </>
  );
}

function CurrentWorkflowRunPanel({ run }: { run: CalendarDayRun | null }) {
  return (
    <SectionPanel
      title="Exécution actuelle du calendrier"
      description="Exécution active si elle existe, sinon dernière exécution connue du calendrier."
    >
      {run ? (
        <CalendarRunRow run={run} />
      ) : (
        <SubtleText>Aucune exécution du calendrier disponible.</SubtleText>
      )}
    </SectionPanel>
  );
}

function WorkflowRunColumn({
  runs,
  title,
}: {
  runs: CalendarDayRun[];
  title: string;
}) {
  return (
    <View className="gap-2">
      <MetaText>{title}</MetaText>
      <View className="gap-2">
        {runs.length === 0 ? (
          <SubtleText>Aucune exécution récente.</SubtleText>
        ) : (
          runs
            .slice(0, 4)
            .map((run) => <CalendarRunRow key={run.id} run={run} />)
        )}
      </View>
    </View>
  );
}

function CalendarRunRow({ run }: { run: CalendarDayRun }) {
  const durationMs = run.completedAt
    ? Math.max(
        0,
        new Date(run.completedAt).getTime() - new Date(run.startedAt).getTime(),
      )
    : null;
  const durationLabel =
    durationMs === null ? "en cours" : `${Math.round(durationMs / 1000)}s`;
  const legacyN8nReference = [
    run.n8nExecutionId ? `execution ${run.n8nExecutionId}` : null,
    run.n8nWorkflowName ?? run.n8nWorkflowId,
  ]
    .filter((value): value is string => Boolean(value))
    .join(" - ");

  return (
    <DataRow>
      <View className="gap-1">
        <View className="flex-row flex-wrap items-center gap-2">
          <StateIndicator tone="trace">
            {run.workflowKind === "summary" ? "Résumé" : "Ingestion"}
          </StateIndicator>
          <StateIndicator tone={runStepTone(run.status)}>
            {runStatusLabel(run.status)}
          </StateIndicator>
        </View>
        <MetaText>
          #{run.id} - tentative {run.attempt} - {durationLabel}
        </MetaText>
        <SubtleText>Motif : {run.reason ?? "n/a"}</SubtleText>
        <SubtleText>
          Début : {formatLocalDateTimeWithRaw(run.startedAt)}
        </SubtleText>
        <SubtleText>
          Fin :{" "}
          {run.completedAt
            ? formatLocalDateTimeWithRaw(run.completedAt)
            : "en cours"}
        </SubtleText>
        <SubtleText>
          Moteur : {legacyN8nReference ? "n8n (historique)" : "Rust natif"}
        </SubtleText>
        {legacyN8nReference ? (
          <SubtleText>Référence historique : {legacyN8nReference}</SubtleText>
        ) : null}
        {run.errorMessage ? <SubtleText>{run.errorMessage}</SubtleText> : null}
      </View>
    </DataRow>
  );
}

function runStatusLabel(status: string) {
  if (status === "pending") return "En cours";
  if (status === "error" || status === "failed") return "Erreur";
  if (status === "ready" || status === "audio_ready" || status === "success")
    return "OK";
  if (status === "audio_generating") return "Audio";
  return status;
}

function ArtifactsPanel({
  artifacts,
}: {
  artifacts: CalendarDayRunArtifact[];
}) {
  const sourceAuditArtifacts = artifacts.filter(isNativeSourceAuditArtifact);
  const otherArtifacts = artifacts.filter(
    (artifact) => !isNativeSourceAuditArtifact(artifact),
  );
  const groups = groupArtifacts(otherArtifacts);
  return (
    <SectionPanel
      title="Artefacts du calendrier"
      description="Traces expurgées du service Rust, regroupées par type."
    >
      <View className="gap-3">
        {sourceAuditArtifacts.length > 0 ? (
          <SourceAuditArtifacts artifacts={sourceAuditArtifacts} />
        ) : null}
        {artifacts.length === 0 ? (
          <SubtleText>Aucun artefact disponible.</SubtleText>
        ) : (
          groups.map((group) => (
            <ArtifactGroup key={group.title} group={group} />
          ))
        )}
      </View>
    </SectionPanel>
  );
}

function SourceAuditArtifacts({
  artifacts,
}: {
  artifacts: CalendarDayRunArtifact[];
}) {
  return (
    <View className="gap-2">
      <View className="flex-row flex-wrap items-center gap-2">
        <MetaText>Sources</MetaText>
        <StateIndicator tone="muted">{artifacts.length}</StateIndicator>
      </View>
      <SubtleText>
        Résultats de collecte Rust expurgés : aucun HTML, cookie, en-tête ou
        contenu source n’est exposé.
      </SubtleText>
      {artifacts.map((artifact) => (
        <SourceAuditArtifactBlock artifact={artifact} key={artifact.id} />
      ))}
    </View>
  );
}

function SourceAuditArtifactBlock({
  artifact,
}: {
  artifact: CalendarDayRunArtifact;
}) {
  const rows = sourceAuditRows(artifact);

  if (rows.length === 0) {
    return <ArtifactBlock artifact={artifact} />;
  }

  return (
    <DataRow>
      <View className="gap-2">
        <View className="flex-row flex-wrap items-center gap-2">
          <StateIndicator tone="trace">sources</StateIndicator>
          <MetaText>{artifact.label}</MetaText>
        </View>
        <SubtleText>
          {formatLocalDateTimeWithRaw(artifact.createdAt)}
        </SubtleText>
        {rows.map((row, index) => (
          <View className="gap-1" key={`${artifact.id}-${row.source}-${index}`}>
            <View className="flex-row flex-wrap items-center gap-2">
              <StateIndicator tone="muted">
                {sourceAuditSourceLabel(row.source)}
              </StateIndicator>
              <StateIndicator tone={sourceAuditOutcomeTone(row.outcome)}>
                {sourceAuditOutcomeLabel(row.outcome)}
              </StateIndicator>
              <MetaText>HTTP {row.httpStatus ?? "n/a"}</MetaText>
            </View>
            {row.detail ? <SubtleText>{row.detail}</SubtleText> : null}
            <MetaText>
              sha256={row.rawSha256 ?? "n/a"} • taille=
              {formatBytes(row.rawLengthBytes)}
            </MetaText>
          </View>
        ))}
        <ArtifactJsonDetails contentJson={artifact.contentJson} />
      </View>
    </DataRow>
  );
}

function ArtifactGroup({
  group,
}: {
  group: { title: string; artifacts: CalendarDayRunArtifact[] };
}) {
  if (group.artifacts.length === 0) {
    return null;
  }

  return (
    <View className="gap-2">
      <View className="flex-row flex-wrap items-center gap-2">
        <MetaText>{group.title}</MetaText>
        <StateIndicator tone="muted">{group.artifacts.length}</StateIndicator>
      </View>
      {group.artifacts.map((artifact) => (
        <ArtifactBlock key={artifact.id} artifact={artifact} />
      ))}
    </View>
  );
}

function ArtifactBlock({ artifact }: { artifact: CalendarDayRunArtifact }) {
  const tokens = useThemeTokens();
  const summary = artifactSummary(artifact);

  return (
    <DataRow>
      <View className="gap-2">
        <View className="flex-row flex-wrap items-center gap-2">
          <StateIndicator tone="trace">{artifact.kind}</StateIndicator>
          <Text
            className="min-w-0 flex-1 text-sm font-bold"
            style={{ color: tokens.colors.textPrimary }}
          >
            {artifact.label}
          </Text>
        </View>
        <SubtleText>
          {formatLocalDateTimeWithRaw(artifact.createdAt)}
        </SubtleText>
        <SubtleText>{summary}</SubtleText>
        <ArtifactJsonDetails contentJson={artifact.contentJson} />
      </View>
    </DataRow>
  );
}

function ArtifactJsonDetails({ contentJson }: { contentJson: string }) {
  const dashboardTheme = useThemeTokens();
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <>
      <Button
        className="self-start"
        onPress={() => setIsExpanded((current) => !current)}
      >
        {isExpanded ? "Masquer le JSON" : "Voir le JSON"}
      </Button>
      {isExpanded ? (
        <ScrollView
          className="max-h-[360px] border px-3 py-3"
          style={{
            backgroundColor: dashboardTheme.colors.surface,
            borderColor: dashboardTheme.colors.border,
            borderRadius: dashboardTheme.radii.panel,
          }}
        >
          <Text
            className="text-sm leading-[18px]"
            style={{
              color: dashboardTheme.colors.textPrimary,
              fontFamily: "monospace",
            }}
          >
            {formatArtifactContent(contentJson)}
          </Text>
        </ScrollView>
      ) : null}
    </>
  );
}

function NewsUsedPanel({
  snapshot,
}: {
  snapshot: AdminCalendarDayToday | null;
}) {
  const tokens = useThemeTokens();
  const [filter, setFilter] = useState<CalendarItemFilter>("all");
  const items = snapshot?.items ?? [];
  const audiosByItemId = useMemo(() => {
    const grouped = new Map<number, CalendarDayItemAudio[]>();
    for (const audio of snapshot?.audios ?? []) {
      const current = grouped.get(audio.calendarItemId) ?? [];
      current.push(audio);
      grouped.set(audio.calendarItemId, current);
    }
    return grouped;
  }, [snapshot?.audios]);
  const filteredItems = useMemo(() => {
    const sorted = items.slice().sort((left, right) => {
      const importanceDelta =
        importanceSortValue(right.finalImportance) -
        importanceSortValue(left.finalImportance);
      if (importanceDelta !== 0) return importanceDelta;
      return eventTimeSortValue(left) - eventTimeSortValue(right);
    });
    return sorted.filter((item) =>
      itemMatchesFilter(item, audiosByItemId, filter),
    );
  }, [audiosByItemId, filter, items]);
  const highCount = items.filter(
    (item) => item.finalImportance === "high",
  ).length;
  const audioReadyCount = items.filter((item) =>
    (audiosByItemId.get(item.id) ?? []).some(
      (audio) => audio.status === "audio_ready" || Boolean(audio.playlistUrl),
    ),
  ).length;
  const audioErrorCount = items.filter((item) =>
    (audiosByItemId.get(item.id) ?? []).some(
      (audio) => audio.status === "error" || Boolean(audio.errorMessage),
    ),
  ).length;
  const filters: Array<{
    id: CalendarItemFilter;
    label: string;
    count: number;
  }> = [
    { id: "all", label: "Tous", count: items.length },
    { id: "high", label: "Importance élevée", count: highCount },
    { id: "audio_ready", label: "Avec audio", count: audioReadyCount },
    { id: "audio_error", label: "Erreur audio", count: audioErrorCount },
  ];

  return (
    <SectionPanel
      title="Événements fusionnés"
      description={`Événements du calendrier : ${items.length}. Ceux d’importance élevée sont affichés en priorité.`}
    >
      <View className="gap-3">
        <View className="flex-row flex-wrap gap-2">
          {filters.map((entry) => (
            <Button
              key={entry.id}
              className="min-w-[110px]"
              onPress={() => setFilter(entry.id)}
              variant={filter === entry.id ? "primary" : "secondary"}
            >
              {entry.label} ({entry.count})
            </Button>
          ))}
        </View>
        {items.length === 0 ? (
          <SubtleText>Aucun événement du calendrier chargé.</SubtleText>
        ) : filteredItems.length === 0 ? (
          <SubtleText>
            Aucun événement ne correspond au filtre actif.
          </SubtleText>
        ) : (
          filteredItems.slice(0, 40).map((item) => (
            <DataRow key={item.id}>
              <View className="gap-1">
                <View className="flex-row flex-wrap items-center gap-2">
                  <StateIndicator
                    tone={
                      item.finalImportance === "high" ? "critical" : "muted"
                    }
                  >
                    {item.finalImportance}
                  </StateIndicator>
                  <MetaText>
                    #{item.id} -{" "}
                    {formatLocalDateTimeWithRaw(item.eventTime, "horaire n/a")}
                  </MetaText>
                </View>
                <Text
                  className="text-sm font-bold"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {item.translatedTitle || item.sourceTitle}
                </Text>
                <SubtleText>
                  {item.category} - confiance{" "}
                  {formatConfidence(item.confidence)}
                </SubtleText>
                {item.reasoning ? (
                  <SubtleText>{item.reasoning}</SubtleText>
                ) : null}
                <ItemAudioStatus audios={audiosByItemId.get(item.id) ?? []} />
              </View>
            </DataRow>
          ))
        )}
      </View>
    </SectionPanel>
  );
}

function FinalTextsPanel({
  snapshot,
}: {
  snapshot: AdminCalendarDayToday | null;
}) {
  const summary = snapshot?.summary;

  return (
    <SectionPanel title="Textes finaux">
      <TextBlock label="Titre" value={summary?.title} />
      <TextBlock label="Résumé du calendrier" value={summary?.summaryText} />
      <TextBlock label="Texte TTS" value={summary?.narrationText} />
    </SectionPanel>
  );
}

function TextBlock({ label, value }: { label: string; value?: string | null }) {
  return (
    <View className="w-full max-w-full min-w-0 gap-1">
      <MetaText>{label}</MetaText>
      <SubtleText>{value || "n/a"}</SubtleText>
    </View>
  );
}

function InconsistenciesPanel({
  snapshot,
}: {
  snapshot: AdminCalendarDayToday | null;
}) {
  const errorArtifacts = (snapshot?.artifacts ?? []).filter((artifact) =>
    artifactLooksLikeError(artifact),
  );
  const messages = [
    snapshot?.state.lastError,
    snapshot?.latestRun?.errorMessage,
    snapshot?.summary?.errorMessage,
  ].filter((value): value is string => Boolean(value));

  if (messages.length === 0 && errorArtifacts.length === 0) {
    return null;
  }

  return (
    <SectionPanel
      title="Incohérences et erreurs"
      description="Erreurs contrôlées remontées par le service du calendrier."
    >
      <View className="gap-3">
        {messages.map((message, index) => (
          <ErrorState key={`${message}-${index}`} message={message} />
        ))}
        {errorArtifacts.slice(0, 3).map((artifact) => (
          <DataRow key={artifact.id}>
            <View className="gap-1">
              <View className="flex-row flex-wrap items-center gap-2">
                <StateIndicator tone="critical">Erreur</StateIndicator>
                <MetaText>{artifact.label}</MetaText>
              </View>
              <SubtleText>{artifactSummary(artifact)}</SubtleText>
            </View>
          </DataRow>
        ))}
      </View>
    </SectionPanel>
  );
}

type ArtifactGroupModel = {
  title: string;
  artifacts: CalendarDayRunArtifact[];
};

function groupArtifacts(
  artifacts: CalendarDayRunArtifact[],
): ArtifactGroupModel[] {
  const groups: ArtifactGroupModel[] = [
    { title: "Erreurs", artifacts: [] },
    { title: "Sources", artifacts: [] },
    { title: "Éléments", artifacts: [] },
    { title: "Prompts", artifacts: [] },
    { title: "OpenRouter", artifacts: [] },
    { title: "Traces historiques", artifacts: [] },
    { title: "Autres", artifacts: [] },
  ];

  for (const artifact of artifacts) {
    const haystack = `${artifact.kind} ${artifact.label}`.toLowerCase();
    if (artifactLooksLikeError(artifact)) {
      groups[0].artifacts.push(artifact);
    } else if (haystack.includes("source")) {
      groups[1].artifacts.push(artifact);
    } else if (haystack.includes("item") || haystack.includes("calendar")) {
      groups[2].artifacts.push(artifact);
    } else if (haystack.includes("prompt")) {
      groups[3].artifacts.push(artifact);
    } else if (
      haystack.includes("openrouter") ||
      haystack.includes("model_output") ||
      haystack.includes("ai")
    ) {
      groups[4].artifacts.push(artifact);
    } else if (
      haystack.includes("callback") ||
      haystack.includes("webhook") ||
      haystack.includes("n8n")
    ) {
      groups[5].artifacts.push(artifact);
    } else {
      groups[6].artifacts.push(artifact);
    }
  }

  return groups.filter((group) => group.artifacts.length > 0);
}

type SourceAuditRow = {
  detail: string | null;
  httpStatus: number | null;
  outcome: string;
  rawLengthBytes: number | null;
  rawSha256: string | null;
  source: string;
};

function isNativeSourceAuditArtifact(artifact: CalendarDayRunArtifact) {
  return artifact.label === "calendar_native_source_audits";
}

function sourceAuditRows(artifact: CalendarDayRunArtifact): SourceAuditRow[] {
  const parsed = safeParseJson(artifact.contentJson);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    return [];
  }
  const sources = (parsed as Record<string, unknown>).sources;
  if (!Array.isArray(sources)) {
    return [];
  }

  return sources.flatMap((source) => {
    if (!source || typeof source !== "object" || Array.isArray(source)) {
      return [];
    }
    const record = source as Record<string, unknown>;
    const sourceName = stringFromUnknown(record.source);
    if (!sourceName) {
      return [];
    }
    return [
      {
        detail: stringFromUnknown(record.detail),
        httpStatus: numberFromUnknown(record.httpStatus),
        outcome: stringFromUnknown(record.outcome) ?? "unknown",
        rawLengthBytes: numberFromUnknown(record.rawLengthBytes),
        rawSha256: stringFromUnknown(record.rawSha256),
        source: sourceName,
      },
    ];
  });
}

function sourceAuditSourceLabel(source: string) {
  if (source === "forex_factory_export") return "ForexFactory JSON";
  if (source === "bea_json") return "BEA JSON";
  if (source === "bea_ics") return "BEA ICS";
  if (source === "fed_fomc") return "Federal Reserve FOMC";
  if (source === "fed_monetary_rss") return "Federal Reserve RSS monétaire";
  if (source === "fed_speeches_rss") return "Federal Reserve RSS discours";
  if (source === "eurostat_ics") return "Eurostat ICS";
  if (source === "ecb_calendar") return "BCE";
  return source;
}

function sourceAuditOutcomeLabel(outcome: string) {
  if (outcome === "covered") return "couverte";
  if (outcome === "unavailable") return "indisponible";
  if (outcome === "refused") return "refusée";
  return outcome;
}

function sourceAuditOutcomeTone(outcome: string): Tone {
  if (outcome === "covered") return "info";
  if (outcome === "refused") return "critical";
  if (outcome === "unavailable") return "muted";
  return "trace";
}

function artifactLooksLikeError(artifact: CalendarDayRunArtifact) {
  const haystack = `${artifact.kind} ${artifact.label}`.toLowerCase();
  return (
    haystack.includes("error") ||
    haystack.includes("failure") ||
    haystack.includes("rejection") ||
    haystack.includes("mismatch") ||
    haystack.includes("incoher")
  );
}

function artifactSummary(artifact: CalendarDayRunArtifact) {
  const parsed = safeParseJson(artifact.contentJson);
  if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
    const record = parsed as Record<string, unknown>;
    const reason = stringFromUnknown(
      record.reason ?? record.errorMessage ?? record.message,
    );
    if (reason) return truncateDebug(reason);
    const normalizedItemCount = numberFromUnknown(record.normalizedItemCount);
    const normalizedHighCount = numberFromUnknown(record.normalizedHighCount);
    if (normalizedItemCount !== null || normalizedHighCount !== null) {
      return `Éléments normalisés : ${normalizedItemCount ?? "n/a"} - high: ${
        normalizedHighCount ?? "n/a"
      }`;
    }
    const dbItemCount = numberFromUnknown(record.dbItemCount);
    const dbHighItemCount = numberFromUnknown(record.dbHighItemCount);
    if (dbItemCount !== null || dbHighItemCount !== null) {
      return `BDD items: ${dbItemCount ?? "n/a"} - high: ${
        dbHighItemCount ?? "n/a"
      }`;
    }
    const choices = record.choices;
    if (Array.isArray(choices)) {
      const firstChoice = choices[0] as Record<string, unknown> | undefined;
      const message = firstChoice?.message as
        Record<string, unknown> | undefined;
      const content = stringFromUnknown(message?.content);
      if (content) return truncateDebug(content);
    }
    return `Champs : ${Object.keys(record).slice(0, 8).join(", ") || "n/a"}`;
  }
  if (Array.isArray(parsed)) {
    return `Entrées dans l’artefact : ${parsed.length}.`;
  }
  return truncateDebug(formatArtifactContent(artifact.contentJson));
}

function safeParseJson(value: string) {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return null;
  }
}

function stringFromUnknown(value: unknown) {
  return typeof value === "string" && value.trim().length > 0
    ? value.trim()
    : null;
}

function numberFromUnknown(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function truncateDebug(value: string, maxLength = 240) {
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized.length > maxLength
    ? `${normalized.slice(0, maxLength - 1)}…`
    : normalized;
}

type CalendarItemFilter = "all" | "high" | "audio_ready" | "audio_error";

function itemMatchesFilter(
  item: CalendarDayItem,
  audiosByItemId: Map<number, CalendarDayItemAudio[]>,
  filter: CalendarItemFilter,
) {
  const audios = audiosByItemId.get(item.id) ?? [];
  if (filter === "all") return true;
  if (filter === "high") return item.finalImportance === "high";
  if (filter === "audio_ready") {
    return audios.some(
      (audio) => audio.status === "audio_ready" || Boolean(audio.playlistUrl),
    );
  }
  return audios.some(
    (audio) => audio.status === "error" || Boolean(audio.errorMessage),
  );
}

function ItemAudioStatus({ audios }: { audios: CalendarDayItemAudio[] }) {
  if (audios.length === 0) {
    return <SubtleText>Audio : non préparé</SubtleText>;
  }

  return (
    <View className="gap-1">
      {audios.slice(0, 3).map((audio) => (
        <View key={audio.id} className="gap-1">
          <View className="flex-row flex-wrap items-center gap-2">
            <StateIndicator tone={audioStatusTone(audio)}>
              {audioStatusLabel(audio)}
            </StateIndicator>
            <MetaText>{audio.audioKind}</MetaText>
          </View>
          {audio.playlistUrl ? (
            <SubtleText>{audio.playlistUrl}</SubtleText>
          ) : null}
          {audio.errorMessage ? (
            <SubtleText>{audio.errorMessage}</SubtleText>
          ) : null}
        </View>
      ))}
    </View>
  );
}

function audioStatusTone(audio: CalendarDayItemAudio) {
  if (audio.status === "error" || audio.errorMessage)
    return "critical" as const;
  if (audio.status === "audio_ready" || audio.playlistUrl)
    return "info" as const;
  if (audio.status === "pending" || audio.status === "audio_generating")
    return "trace" as const;
  return "muted" as const;
}

function audioStatusLabel(audio: CalendarDayItemAudio) {
  if (audio.status === "error" || audio.errorMessage) return "Erreur audio";
  if (audio.status === "audio_ready" || audio.playlistUrl) return "Audio prêt";
  if (audio.status === "pending") return "En attente";
  if (audio.status === "audio_generating") return "Génération";
  return audio.status;
}

function importanceSortValue(importance: string) {
  if (importance === "high") return 3;
  if (importance === "medium") return 2;
  if (importance === "low") return 1;
  return 0;
}

function eventTimeSortValue(item: CalendarDayItem) {
  if (!item.eventTime) return Number.MAX_SAFE_INTEGER;
  const value = new Date(item.eventTime).getTime();
  return Number.isFinite(value) ? value : Number.MAX_SAFE_INTEGER;
}

function formatConfidence(confidence: number | null) {
  if (confidence === null) return "n/a";
  return `${Math.round(confidence * 100)}%`;
}
