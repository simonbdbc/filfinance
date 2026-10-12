import { useState } from "react";
import { Platform } from "react-native";

import { useAuth } from "@features/auth";
import { useAudioRuntime } from "@features/audio";
import { useNewsHistoryRuntime } from "@features/history";
import { useLiveMetadataRuntime } from "@features/flux-status";
import { useUserProfileRuntime } from "@features/profile";
import { useTestRunRuntime } from "@features/test-runs";
import { defaultWsUrl, deriveTraceWsUrl } from "@shared/api/urls";
import { appRoleDisplayLabel, canAccessAdmin } from "@shared/auth/roles";
import { useTheme } from "@shared/theme/theme";

export function useAppRuntimeValue() {
  const { authSession } = useAuth();
  const { setThemeName } = useTheme();
  const isWebPlatform = Platform.OS === "web";
  const [wsUrlInput, setWsUrlInput] = useState(() => defaultWsUrl());
  const [wsUrl, setWsUrl] = useState(() => defaultWsUrl());
  const [newsConnectionVersion, setNewsConnectionVersion] = useState(0);
  const traceWsUrl = deriveTraceWsUrl(wsUrl);
  const userProfile = useUserProfileRuntime({
    authSession,
    wsUrl,
  });
  const newsHistoryRuntime = useNewsHistoryRuntime({
    authSession,
    reloadVersion: newsConnectionVersion,
    wsUrl,
  });
  const audioRuntime = useAudioRuntime({
    isWebPlatform,
    wsUrl,
    authSession,
    calendarDaySummaryStatus: newsHistoryRuntime.calendarDaySummaryStatus,
    historyRef: newsHistoryRuntime.historyRef,
    dailyFeedHistoryRef: newsHistoryRuntime.dailyFeedHistoryRef,
    dailyTestRunHistoryRef: newsHistoryRuntime.dailyTestRunHistoryRef,
    lastItemRef: newsHistoryRuntime.lastItemRef,
    latestKnownItem: newsHistoryRuntime.latestKnownItem,
  });
  const canUseTestRunSocket =
    userProfile.isUserProfileCurrent &&
    canAccessAdmin(userProfile.visibleUserProfileRole);
  const currentRoleLabel = appRoleDisplayLabel(userProfile.userProfileRole);
  const testRunUnavailableReason =
    userProfile.isUserProfileCurrent && currentRoleLabel
      ? `Rôle utilisateur courant : ${currentRoleLabel}. Le canal de test nécessite le rôle modérateur ou administrateur.`
      : "Chargement du profil utilisateur avant l’ouverture du canal de test.";
  const testRunRuntime = useTestRunRuntime({
    traceWsUrl,
    authSession,
    isEnabled: canUseTestRunSocket,
    onBeforeTrigger: audioRuntime.resetAudioForTestRun,
    testRunUnavailableReason,
    findDeliveredTestItem: newsHistoryRuntime.findDeliveredTestItem,
  });
  const {
    connectionState,
    lastLaggedCount,
    lastError,
    activeLiveMetadataWsUrl,
    isLiveMetadataEdgeEnabled,
    isLiveMetadataFallbackActive,
    resetLiveMetadataState,
  } = useLiveMetadataRuntime({
    wsUrl,
    authSession,
    reloadVersion: newsConnectionVersion,
    onAudioReady: audioRuntime.handleEdgeAudioReady,
    onCalendarAlertReady: audioRuntime.handleCalendarAlertReady,
    onCalendarDaySummary: newsHistoryRuntime.recordCalendarDaySummaryStatus,
    onAudioFailed: audioRuntime.handleEdgeAudioFailed,
    onAudioFinalized: audioRuntime.handleEdgeAudioFinalized,
    onSourceStatuses: newsHistoryRuntime.recordNewsSourceStatuses,
    onTranslatedNewsItem: newsHistoryRuntime.recordTranslatedNewsItem,
  });
  const reconnectWithWsUrlInput = () => {
    audioRuntime.resetAudioForReconnect();
    newsHistoryRuntime.resetNewsHistoryRuntime();
    testRunRuntime.resetTraceState();
    resetLiveMetadataState();
    userProfile.resetUserProfileState();
    setWsUrl(wsUrlInput.trim());
    setNewsConnectionVersion((currentVersion) => currentVersion + 1);
  };
  const isDailyAudioReplayActive =
    audioRuntime.dailySummaryPlaybackState !== "idle" ||
    (audioRuntime.audioTransportMode === "hls" &&
      (audioRuntime.hlsPlaybackMode === "daily_summary" ||
        audioRuntime.hlsPlaybackMode === "calendar_summary") &&
      (audioRuntime.audioConnectionState === "connecting" ||
        audioRuntime.audioConnectionState === "waiting" ||
        audioRuntime.audioConnectionState === "open"));
  const toggleDailySummaryReplay = () => {
    if (isDailyAudioReplayActive) {
      audioRuntime.stopAudioPlayback();
      return;
    }

    void audioRuntime.replayAudioForHistory(
      newsHistoryRuntime.dailyFeedHistory,
      "le résumé de la journée",
    );
  };

  return {
    role: userProfile.visibleUserProfileRole,
    isUserProfileCurrent: userProfile.isUserProfileCurrent,
    canAccessAdmin: canAccessAdmin(userProfile.visibleUserProfileRole),
    theme: {
      setThemeName,
    },
    server: {
      wsUrl,
    },
    audioControl: {
      isWebPlatform,
      isAudioEnabled: audioRuntime.isAudioEnabled,
      audioConnectionState: audioRuntime.audioConnectionState,
      audioTransportMode: audioRuntime.audioTransportMode,
      hlsPlaybackMode: audioRuntime.hlsPlaybackMode,
      bufferedAudioClipCount: audioRuntime.bufferedAudioClipCount,
      dailySummaryPlaybackState: audioRuntime.dailySummaryPlaybackState,
      dailySummaryAvailabilityStatus:
        audioRuntime.dailySummaryAvailabilityStatus,
      isDailyAudioReplayActive,
      onAudioEnabledChange: audioRuntime.changeAudioEnabled,
      onDailySummaryReplayPress: toggleDailySummaryReplay,
    },
    errors: {
      fluxError: lastError,
      audioError: audioRuntime.lastAudioError,
      historyError: newsHistoryRuntime.dailyHistoryError,
      traceError: testRunRuntime.lastTraceError,
      profileError: userProfile.userProfileError,
    },
    live: {
      connectionState,
      wsUrlInput,
      setWsUrlInput,
      reconnectWithWsUrlInput,
      isWebPlatform,
      isAudioEnabled: audioRuntime.isAudioEnabled,
      changeAudioEnabled: audioRuntime.changeAudioEnabled,
      audioConnectionState: audioRuntime.audioConnectionState,
      lastAudioLaggedCount: audioRuntime.lastAudioLaggedCount,
      bufferedAudioClipCount: audioRuntime.bufferedAudioClipCount,
      effectiveAudioVoice: audioRuntime.effectiveAudioVoice,
      audioPlaybackEngine: audioRuntime.audioPlaybackEngine,
      audioTransportMode: audioRuntime.audioTransportMode,
      hlsPlaybackMode: audioRuntime.hlsPlaybackMode,
      hlsReplayTodayMetadata: audioRuntime.hlsReplayTodayMetadata,
      hlsAudioUrl: audioRuntime.hlsAudioUrl,
      hlsReplayTodayPlaylistUrl: audioRuntime.hlsReplayTodayPlaylistUrl,
      activeLiveMetadataWsUrl,
      isLiveMetadataEdgeEnabled,
      isLiveMetadataFallbackActive,
      availableVoices: audioRuntime.availableVoices,
      selectedVoice: audioRuntime.selectedVoice,
      pendingVoiceSelection: audioRuntime.pendingVoiceSelection,
      canChangeVoice: audioRuntime.canChangeVoice,
      isUpdatingVoice: audioRuntime.isUpdatingVoice,
      requestVoiceChange: audioRuntime.requestVoiceChange,
      currentItem: audioRuntime.currentItem,
      currentOrLatestItem: audioRuntime.currentOrLatestItem,
      queuedCount: audioRuntime.queuedCount,
      laggedCount: lastLaggedCount,
      stopAudioPlayback: audioRuntime.stopAudioPlayback,
      clearCurrentAndQueue: audioRuntime.clearCurrentAndQueue,
      clearCurrentItem: audioRuntime.clearCurrentItem,
    },
    history: {
      dailyFeedHistory: newsHistoryRuntime.dailyFeedHistory,
      dailyTestRunHistory: newsHistoryRuntime.dailyTestRunHistory,
      isDailyHistoryLoading: newsHistoryRuntime.isDailyHistoryLoading,
      currentLocalDayKey: newsHistoryRuntime.currentLocalDayKey,
      previousLocalDayKey: newsHistoryRuntime.previousLocalDayKey,
      isPreviousFeedDayVisible: newsHistoryRuntime.isPreviousFeedDayVisible,
      setIsPreviousFeedDayVisible:
        newsHistoryRuntime.setIsPreviousFeedDayVisible,
      dailySummaryPlaybackState: audioRuntime.dailySummaryPlaybackState,
      dailySummaryAvailabilityStatus:
        audioRuntime.dailySummaryAvailabilityStatus,
      dailySummaryAvailabilityMessage:
        audioRuntime.dailySummaryAvailabilityMessage,
      isDailyAudioReplayActive,
      newsSourceStatuses: newsHistoryRuntime.newsSourceStatuses,
      replayAudioForHistory: audioRuntime.replayAudioForHistory,
      stopAudioPlayback: audioRuntime.stopAudioPlayback,
    },
    tests: {
      canUseTestRunSocket: testRunRuntime.canUseTestRunSocket,
      testRunUnavailableReason: testRunRuntime.testRunUnavailableReason,
      traceConnectionState: testRunRuntime.traceConnectionState,
      traceWsUrl: testRunRuntime.traceWsUrl,
      isAudioEnabled: audioRuntime.isAudioEnabled,
      activeTestRun: testRunRuntime.activeTestRun,
      lastTraceLaggedCount: testRunRuntime.lastTraceLaggedCount,
      isTriggeringTestRun: testRunRuntime.isTriggeringTestRun,
      lastTraceItem: testRunRuntime.lastTraceItem,
      deliveredTestItem: testRunRuntime.deliveredTestItem,
      triggerTestRun: testRunRuntime.triggerTestRun,
      testTraceItems: testRunRuntime.testTraceItems,
      isTracePanelCollapsed: testRunRuntime.isTracePanelCollapsed,
      setIsTracePanelCollapsed: testRunRuntime.setIsTracePanelCollapsed,
      isTestRunHistoryCollapsed: testRunRuntime.isTestRunHistoryCollapsed,
      setIsTestRunHistoryCollapsed: testRunRuntime.setIsTestRunHistoryCollapsed,
    },
    profile: {
      visibleUserProfile: userProfile.visibleUserProfile,
      isUserProfileCurrent: userProfile.isUserProfileCurrent,
      userProfileError: userProfile.userProfileError,
    },
  };
}
