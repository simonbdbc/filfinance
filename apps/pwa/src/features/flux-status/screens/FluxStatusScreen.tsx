import { useWindowDimensions } from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { Grid, KpiStrip, WorkspacePage, Section } from "@shared/ui/Workspace";

import { ConnectionSettingsCard } from "@features/settings";
import { TestRunsPanel } from "@features/test-runs";
import { connectionLabel } from "@shared/lib/status";
import { AccessDeniedCard, UserProfileLoadingCard } from "@shared/ui";
import { CurrentLiveNewsCard } from "../components/CurrentLiveNewsCard";
import { LiveRuntimeStatsRow } from "../components/LiveRuntimeStatsRow";

export function FluxStatusScreen() {
  const { canAccessAdmin, history, live, profile, role, tests } =
    useAppRuntime();
  const { width } = useWindowDimensions();
  const isWideLayout = width >= 1080;

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  const laggedCount = live.laggedCount ?? 0;

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminFluxStatus}
      description="Supervision du transport en direct, de l’audio et des tests d’exploitation."
    >
      <KpiStrip
        items={[
          {
            label: "Flux en direct",
            value: connectionLabel(live.connectionState),
            tone: live.connectionState === "open" ? "info" : "trace",
          },
          {
            label: "File",
            value: live.queuedCount,
            tone: live.queuedCount > 0 ? "trace" : "muted",
          },
          {
            label: "Retard",
            value: laggedCount,
            tone: laggedCount > 0 ? "critical" : "muted",
          },
          {
            label: "Clips audio",
            value: live.bufferedAudioClipCount,
            tone: live.bufferedAudioClipCount > 0 ? "info" : "muted",
          },
          {
            label: "Tests",
            value: tests.canUseTestRunSocket ? "prêt" : "bloqué",
            tone: tests.canUseTestRunSocket ? "info" : "critical",
          },
        ]}
      />

      <Grid
        direction={isWideLayout ? "horizontal" : "vertical"}
        minItemWidth={420}
      >
        <ConnectionSettingsCard
          connectionState={live.connectionState}
          wsUrlInput={live.wsUrlInput}
          onWsUrlInputChange={live.setWsUrlInput}
          onReconnect={live.reconnectWithWsUrlInput}
          isWebPlatform={live.isWebPlatform}
          isAudioEnabled={live.isAudioEnabled}
          onAudioEnabledChange={live.changeAudioEnabled}
          audioConnectionState={live.audioConnectionState}
          lastAudioLaggedCount={live.lastAudioLaggedCount}
          bufferedAudioClipCount={live.bufferedAudioClipCount}
          effectiveAudioVoice={live.effectiveAudioVoice}
          audioPlaybackEngine={live.audioPlaybackEngine}
          audioTransportMode={live.audioTransportMode}
          hlsPlaybackMode={live.hlsPlaybackMode}
          hlsReplayTodayMetadata={live.hlsReplayTodayMetadata}
          hlsAudioUrl={live.hlsAudioUrl}
          hlsReplayTodayPlaylistUrl={live.hlsReplayTodayPlaylistUrl}
          liveMetadataWsUrl={live.activeLiveMetadataWsUrl}
          isLiveMetadataEdgeEnabled={live.isLiveMetadataEdgeEnabled}
          isLiveMetadataFallbackActive={live.isLiveMetadataFallbackActive}
          availableVoices={live.availableVoices}
          selectedVoice={live.selectedVoice}
          pendingVoiceSelection={live.pendingVoiceSelection}
          canChangeVoice={live.canChangeVoice}
          isUpdatingVoice={live.isUpdatingVoice}
          onVoiceChange={live.requestVoiceChange}
        />
        <Grid direction="vertical">
          <LiveRuntimeStatsRow
            currentItem={live.currentItem}
            queuedCount={live.queuedCount}
            laggedCount={live.laggedCount}
          />
          <CurrentLiveNewsCard
            currentItem={live.currentItem}
            currentOrLatestItem={live.currentOrLatestItem}
            onStop={live.clearCurrentItem}
            onClearQueue={live.clearCurrentAndQueue}
          />
        </Grid>
      </Grid>

      <Section
        title="Tests"
        description="Validation contrôlée du flux, des traces et de l’historique audio."
      >
        <TestRunsPanel
          controlProps={{
            traceConnectionState: tests.traceConnectionState,
            traceWsUrl: tests.traceWsUrl,
            isAudioEnabled: tests.isAudioEnabled,
            activeTestRun: tests.activeTestRun,
            lastTraceLaggedCount: tests.lastTraceLaggedCount,
            isTriggeringTestRun: tests.isTriggeringTestRun,
            canTriggerTestRun: tests.canUseTestRunSocket,
            unavailableReason: tests.testRunUnavailableReason,
            lastTraceItem: tests.lastTraceItem,
            testTraceItems: tests.testTraceItems,
            deliveredTestItem: tests.deliveredTestItem,
            onTriggerTestRun: tests.triggerTestRun,
          }}
          traceProps={{
            items: tests.testTraceItems,
            lastTraceItem: tests.lastTraceItem,
            isCollapsed: tests.isTracePanelCollapsed,
            onCollapsedChange: tests.setIsTracePanelCollapsed,
          }}
          historyProps={{
            items: history.dailyTestRunHistory,
            isLoading: history.isDailyHistoryLoading,
            currentLocalDayKey: history.currentLocalDayKey,
            previousLocalDayKey: history.previousLocalDayKey,
            isCollapsed: tests.isTestRunHistoryCollapsed,
            onCollapsedChange: tests.setIsTestRunHistoryCollapsed,
            onReplayAudio: (items, historyLabel) => {
              void history.replayAudioForHistory(items, historyLabel);
            },
          }}
        />
      </Section>
    </WorkspacePage>
  );
}
