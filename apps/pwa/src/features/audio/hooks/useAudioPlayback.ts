import type { AuthSessionClient } from "@shared/api/authSession";
import {
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
  useEffect,
  useRef,
  useState,
} from "react";

import {
  audioTimelineItemFromNewsItem,
  nextLastReceivedAudioTimelineItem,
  pruneRecentCompletedAudioIds,
  shouldAdmitAudioItem,
} from "../services/audioQueue";
import { loadDailySummaryToday } from "../services/dailySummaryApi";
import { useHlsAudioPlayer } from "./useHlsAudioPlayer";
import type {
  AudioTransportMode,
  EdgeAudioFailedMessage,
  EdgeAudioFinalizedMessage,
  EdgeAudioReadyMessage,
  EdgeCalendarAlertReadyMessage,
  PublicCalendarDaySummaryStatus,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import {
  buildDailySummaryTodayUrl,
  buildHlsAccessTicketUrl,
  buildReplayTodayMetadataUrl,
  buildReplayTodayPlaylistUrl,
} from "@shared/api/urls";
import {
  type AudioTimelineItem,
  compareAudioTimelineItems,
} from "@shared/lib/audioTimeline";

type UseAudioPlaybackOptions = {
  isWebPlatform: boolean;
  wsUrl: string;
  authSession: AuthSessionClient | null;
  calendarDaySummaryStatus: PublicCalendarDaySummaryStatus | null;
  queueRef: MutableRefObject<TranslatedNewsItem[]>;
  historyRef: MutableRefObject<TranslatedNewsItem[]>;
  dailyFeedHistoryRef: MutableRefObject<TranslatedNewsItem[]>;
  dailyTestRunHistoryRef: MutableRefObject<TranslatedNewsItem[]>;
  lastItemRef: MutableRefObject<TranslatedNewsItem | null>;
  setQueue: Dispatch<SetStateAction<TranslatedNewsItem[]>>;
  setCurrentItem: Dispatch<SetStateAction<TranslatedNewsItem | null>>;
};

type QueuedHlsClip = EdgeAudioReadyMessage["payload"];
type DailySummaryPlaybackState = "idle" | "waiting" | "playing";
type DailySummaryAvailabilityStatus =
  | "loading"
  | "ready"
  | "unavailable"
  | "error";
type AudioReadyIgnoreReason =
  | "audio_disabled"
  | "transport_not_hls"
  | "playback_mode_blocks_live"
  | "missing_playlist_url"
  | "duplicate_stream"
  | "recently_completed"
  | "timeline_watermark";
const REPLAY_TODAY_METADATA_REFRESH_MS = 30_000;

export function useAudioPlayback({
  isWebPlatform,
  wsUrl,
  authSession,
  calendarDaySummaryStatus,
  queueRef,
  historyRef,
  dailyFeedHistoryRef,
  dailyTestRunHistoryRef,
  lastItemRef,
  setQueue,
  setCurrentItem,
}: UseAudioPlaybackOptions) {
  const [isAudioEnabled, setIsAudioEnabled] = useState(false);
  const [audioTransportMode, setAudioTransportMode] =
    useState<AudioTransportMode>("idle");
  const [lastAudioError, setLastAudioError] = useState<string | null>(null);
  const [queuedHlsClipCount, setQueuedHlsClipCount] = useState(0);
  const [dailySummaryPlaybackState, setDailySummaryPlaybackStateState] =
    useState<DailySummaryPlaybackState>("idle");
  const [dailySummaryAvailabilityStatus, setDailySummaryAvailabilityStatus] =
    useState<DailySummaryAvailabilityStatus>("loading");
  const [dailySummaryAvailabilityMessage, setDailySummaryAvailabilityMessage] =
    useState<string | null>(null);
  const lastReceivedAudioTimelineItemRef = useRef<AudioTimelineItem | null>(null);
  const currentAudioTimelineItemRef = useRef<AudioTimelineItem | null>(null);
  const queuedHlsClipsRef = useRef<QueuedHlsClip[]>([]);
  const currentHlsClipRef = useRef<QueuedHlsClip | null>(null);
  const knownHlsClipStreamIdsRef = useRef<Set<string>>(new Set());
  const recentCompletedAudioIdsRef = useRef<Map<number, number>>(new Map());
  const isAudioEnabledRef = useRef(false);
  const audioTransportModeRef = useRef<AudioTransportMode>("idle");
  const dailySummaryPlaybackStateRef =
    useRef<DailySummaryPlaybackState>("idle");
  const dailySummaryAbortControllerRef = useRef<AbortController | null>(null);
  const dailySummaryPlaybackRunIdRef = useRef(0);
  const hlsReplayTodayPlaylistUrl = buildReplayTodayPlaylistUrl(wsUrl);
  const hlsReplayTodayMetadataUrl = buildReplayTodayMetadataUrl(wsUrl);
  const dailySummaryTodayUrl = buildDailySummaryTodayUrl(wsUrl);
  const hlsAccessTicketUrl = buildHlsAccessTicketUrl(wsUrl);

  useEffect(() => {
    isAudioEnabledRef.current = isAudioEnabled;
  }, [isAudioEnabled]);

  useEffect(() => {
    audioTransportModeRef.current = audioTransportMode;
  }, [audioTransportMode]);

  useEffect(() => {
    if (!isWebPlatform) {
      setDailySummaryAvailabilityStatus("unavailable");
      setDailySummaryAvailabilityMessage("Disponible sur le client Web uniquement.");
      return;
    }

    if (!authSession) {
      setDailySummaryAvailabilityStatus("unavailable");
      setDailySummaryAvailabilityMessage("Session requise.");
      return;
    }

    if (!calendarDaySummaryStatus) {
      setDailySummaryAvailabilityStatus("loading");
      setDailySummaryAvailabilityMessage(null);
      return;
    }

    if (calendarDaySummaryStatus.audio_available) {
      setDailySummaryAvailabilityStatus("ready");
      setDailySummaryAvailabilityMessage(null);
      return;
    }

    if (calendarDaySummaryStatus.status === "audio_ready") {
      setDailySummaryAvailabilityStatus("error");
      setDailySummaryAvailabilityMessage(
        "Résumé audio marqué prêt mais aucune playlist HLS n’est disponible.",
      );
      return;
    }

    if (calendarDaySummaryStatus.status === "error") {
      setDailySummaryAvailabilityStatus("error");
      setDailySummaryAvailabilityMessage("Résumé indisponible.");
      return;
    }

    setDailySummaryAvailabilityStatus("unavailable");
    setDailySummaryAvailabilityMessage(
      calendarDaySummaryAvailabilityMessage(calendarDaySummaryStatus.status),
    );
  }, [authSession, calendarDaySummaryStatus, isWebPlatform]);

  function setDailySummaryPlaybackState(nextState: DailySummaryPlaybackState) {
    dailySummaryPlaybackStateRef.current = nextState;
    setDailySummaryPlaybackStateState(nextState);
  }

  function abortDailySummaryPolling() {
    const controller = dailySummaryAbortControllerRef.current;
    dailySummaryAbortControllerRef.current = null;
    controller?.abort();
  }

  const resolveItemForPlayback = (translationId: number) => {
    return (
      queueRef.current.find((item) => item.id === translationId)
      || dailyFeedHistoryRef.current.find((item) => item.id === translationId)
      || dailyTestRunHistoryRef.current.find((item) => item.id === translationId)
      || historyRef.current.find((item) => item.id === translationId)
      || (lastItemRef.current?.id === translationId ? lastItemRef.current : null)
    );
  };

  const canAdmitAudioTimelineItem = (candidate: AudioTimelineItem) =>
    shouldAdmitAudioItem(
      candidate,
      currentAudioTimelineItemRef.current,
      lastReceivedAudioTimelineItemRef.current,
    );

  const recordLastReceivedAudioTimelineItem = (item: AudioTimelineItem) => {
    lastReceivedAudioTimelineItemRef.current = nextLastReceivedAudioTimelineItem(
      lastReceivedAudioTimelineItemRef.current,
      item,
    );
  };

  const markAudioClipCompleted = (translationId: number) => {
    if (translationId <= 0) {
      return;
    }

    pruneRecentCompletedAudioIds(recentCompletedAudioIdsRef.current);
    recentCompletedAudioIdsRef.current.set(translationId, Date.now());
  };

  function clearHlsClipQueue() {
    queuedHlsClipsRef.current = [];
    currentHlsClipRef.current = null;
    knownHlsClipStreamIdsRef.current.clear();
    setQueuedHlsClipCount(0);
  }

  function audioTimelineItemForHlsClip(payload: QueuedHlsClip): AudioTimelineItem {
    const item = resolveItemForPlayback(payload.translation_id);
    if (item) {
      return audioTimelineItemFromNewsItem(item);
    }

    return {
      id: payload.translation_id,
      timestamp: payload.timestamp || payload.published_at,
      created_at: payload.published_at,
    };
  }

  function compareQueuedHlsClipsOldestFirst(left: QueuedHlsClip, right: QueuedHlsClip) {
    return compareAudioTimelineItems(
      audioTimelineItemForHlsClip(left),
      audioTimelineItemForHlsClip(right),
    );
  }

  function enqueueHlsClip(payload: QueuedHlsClip) {
    if (knownHlsClipStreamIdsRef.current.has(payload.stream_id)) {
      reportIgnoredAudioReady("duplicate_stream", payload);
      return false;
    }

    if (recentCompletedAudioIdsRef.current.has(payload.translation_id)) {
      reportIgnoredAudioReady("recently_completed", payload);
      return false;
    }

    const timelineItem = audioTimelineItemForHlsClip(payload);
    if (!canAdmitAudioTimelineItem(timelineItem)) {
      markAudioClipCompleted(payload.translation_id);
      reportIgnoredAudioReady("timeline_watermark", payload, {
        candidate: timelineItem,
        current: currentAudioTimelineItemRef.current,
        lastReceived: lastReceivedAudioTimelineItemRef.current,
      });
      return false;
    }

    knownHlsClipStreamIdsRef.current.add(payload.stream_id);
    queuedHlsClipsRef.current = [...queuedHlsClipsRef.current, payload].sort(
      compareQueuedHlsClipsOldestFirst,
    );
    setQueuedHlsClipCount(queuedHlsClipsRef.current.length);
    return true;
  }

  async function playNextQueuedHlsClip() {
    if (
      !isAudioEnabledRef.current ||
      audioTransportModeRef.current !== "hls" ||
      hlsPlaybackMode === "replay_today" ||
      hlsPlaybackMode === "daily_summary" ||
      hlsPlaybackMode === "calendar_summary" ||
      currentHlsClipRef.current
    ) {
      return;
    }

    const nextClip = queuedHlsClipsRef.current.shift() ?? null;
    setQueuedHlsClipCount(queuedHlsClipsRef.current.length);

    if (!nextClip) {
      stopHlsPlayback();
      setCurrentItem(null);
      currentAudioTimelineItemRef.current = null;
      return;
    }

    const matchingItem = resolveItemForPlayback(nextClip.translation_id);
    const timelineItem = audioTimelineItemForHlsClip(nextClip);
    if (!canAdmitAudioTimelineItem(timelineItem)) {
      markAudioClipCompleted(nextClip.translation_id);
      void playNextQueuedHlsClip();
      return;
    }

    currentHlsClipRef.current = nextClip;
    currentAudioTimelineItemRef.current = timelineItem;
    recordLastReceivedAudioTimelineItem(timelineItem);
    setCurrentItem(matchingItem);

    try {
      await startClipHlsPlayback(nextClip.playlist_url);
      console.info("[audio-live] hls clip playback started", {
        stream_id: nextClip.stream_id,
        translation_id: nextClip.translation_id,
        audio_ready_to_client_playing_ms: elapsedSinceIsoTimestampMs(nextClip.published_at),
      });
      setLastAudioError(null);
    } catch (error) {
      currentHlsClipRef.current = null;
      markAudioClipCompleted(nextClip.translation_id);
      console.warn("[audio-live] hls clip playback failed", {
        stream_id: nextClip.stream_id,
        translation_id: nextClip.translation_id,
        playlist_url: nextClip.playlist_url,
        error,
      });
      setLastAudioError(
        error instanceof Error
          ? `Clip HLS indisponible : ${error.message}`
          : "Clip HLS indisponible",
      );
      void playNextQueuedHlsClip();
    }
  }

  function handleHlsClipEnded() {
    const completedClip = currentHlsClipRef.current;
    currentHlsClipRef.current = null;
    currentAudioTimelineItemRef.current = null;

    if (completedClip) {
      console.info("[audio-live] hls clip playback ended", {
        stream_id: completedClip.stream_id,
        translation_id: completedClip.translation_id,
      });
      markAudioClipCompleted(completedClip.translation_id);
    }

    stopHlsPlayback();
    void playNextQueuedHlsClip();
  }

  function handleReplayTodayEnded() {
    clearHlsClipQueue();
    abortDailySummaryPolling();
    setDailySummaryPlaybackState("idle");
    setAudioTransportMode("idle");
    setIsAudioEnabled(false);
    setCurrentItem(null);
    setQueue([]);
  }

  const {
    hlsConnectionState,
    hlsPlaybackMode,
    hlsReplayTodayMetadata,
    hlsAudioUrl,
    hlsPlaybackEngine,
    isHlsPlaybackActive,
    refreshReplayTodayMetadata,
    startClipHlsPlayback,
    startDailySummaryHlsPlayback,
    stopHlsPlayback,
  } = useHlsAudioPlayer({
    isWebPlatform,
    wsUrl,
    replayTodayPlaylistUrl: hlsReplayTodayPlaylistUrl,
    replayTodayMetadataUrl: hlsReplayTodayMetadataUrl,
    hlsAccessTicketUrl,
    authSession,
    onPlaybackError: setLastAudioError,
    onClipEnded: handleHlsClipEnded,
    onReplayTodayEnded: handleReplayTodayEnded,
  });
  const shouldPollReplayTodayMetadata =
    isAudioEnabled &&
    audioTransportMode === "hls" &&
    hlsPlaybackMode === "replay_today" &&
    isHlsPlaybackActive();

  useEffect(() => {
    if (!isWebPlatform || !authSession) {
      return;
    }

    let isCancelled = false;
    const refreshReplayAvailability = () => {
      void refreshReplayTodayMetadata().catch((error) => {
        if (!isCancelled) {
          console.warn("[audio-live] replay today metadata refresh failed", error);
        }
      });
    };

    // Le replay des clips live garde sa metadata separee du resume IA journalier.
    refreshReplayAvailability();
    if (!shouldPollReplayTodayMetadata) {
      return () => {
        isCancelled = true;
      };
    }

    const intervalId = window.setInterval(
      refreshReplayAvailability,
      REPLAY_TODAY_METADATA_REFRESH_MS,
    );

    return () => {
      isCancelled = true;
      window.clearInterval(intervalId);
    };
  }, [
    authSession,
    isWebPlatform,
    refreshReplayTodayMetadata,
    shouldPollReplayTodayMetadata,
  ]);

  function hlsLivePlaybackIgnoreReason(
    payload: QueuedHlsClip,
  ): AudioReadyIgnoreReason | null {
    if (!isAudioEnabledRef.current) {
      return "audio_disabled";
    }

    if (audioTransportModeRef.current !== "hls") {
      return "transport_not_hls";
    }

    if (
      hlsPlaybackMode === "replay_today" ||
      hlsPlaybackMode === "daily_summary" ||
      hlsPlaybackMode === "calendar_summary"
    ) {
      return "playback_mode_blocks_live";
    }

    if (!normalizeAudioUrl(payload.playlist_url)) {
      return "missing_playlist_url";
    }

    return null;
  }

  function reportIgnoredAudioReady(
    reason: AudioReadyIgnoreReason,
    payload: QueuedHlsClip,
    detail?: Record<string, unknown>,
  ) {
    if (reason === "audio_disabled") {
      return;
    }

    console.info("[audio-live] audio_ready ignored", {
      reason,
      message: audioReadyIgnoreMessage(reason),
      stream_id: payload.stream_id,
      translation_id: payload.translation_id,
      playlist_url: payload.playlist_url,
      published_at: payload.published_at,
      ...detail,
    });
  }

  const handleEdgeAudioReady = (message: EdgeAudioReadyMessage) => {
    const payload = message.payload;
    const ignoreReason = hlsLivePlaybackIgnoreReason(payload);
    if (ignoreReason) {
      reportIgnoredAudioReady(ignoreReason, payload, {
        audioTransportMode: audioTransportModeRef.current,
        hlsPlaybackMode,
      });
      if (
        ignoreReason === "playback_mode_blocks_live" ||
        ignoreReason === "missing_playlist_url" ||
        ignoreReason === "transport_not_hls"
      ) {
        setLastAudioError(audioReadyIgnoreMessage(ignoreReason));
      }
      return;
    }

    if (enqueueHlsClip(payload)) {
      void playNextQueuedHlsClip();
    }
  };

  const handleCalendarAlertReady = (message: EdgeCalendarAlertReadyMessage) => {
    if (
      !isAudioEnabledRef.current ||
      audioTransportModeRef.current !== "hls" ||
      hlsPlaybackMode === "replay_today" ||
      hlsPlaybackMode === "daily_summary" ||
      hlsPlaybackMode === "calendar_summary"
    ) {
      return;
    }

    const alertClip: QueuedHlsClip = {
      event_id: message.payload.event_id,
      server_time: message.payload.server_time,
      translation_id: -message.payload.calendar_item_id,
      news_event_id: null,
      timestamp: message.payload.scheduled_at,
      stream_id: message.payload.stream_id,
      playlist_url: message.payload.playlist_url,
      published_at: message.payload.published_at,
      target_duration_secs: 3,
    };

    if (knownHlsClipStreamIdsRef.current.has(alertClip.stream_id)) {
      return;
    }

    knownHlsClipStreamIdsRef.current.add(alertClip.stream_id);
    queuedHlsClipsRef.current = [alertClip, ...queuedHlsClipsRef.current];
    setQueuedHlsClipCount(queuedHlsClipsRef.current.length);
    void playNextQueuedHlsClip();
  };

  const handleEdgeAudioFailed = (message: EdgeAudioFailedMessage) => {
    const streamId = message.payload.stream_id;
    if (streamId) {
      queuedHlsClipsRef.current = queuedHlsClipsRef.current.filter(
        (clip) => clip.stream_id !== streamId,
      );
      knownHlsClipStreamIdsRef.current.delete(streamId);
      setQueuedHlsClipCount(queuedHlsClipsRef.current.length);

      if (currentHlsClipRef.current?.stream_id === streamId) {
        currentHlsClipRef.current = null;
        stopHlsPlayback();
        void playNextQueuedHlsClip();
      }
    }

    setLastAudioError(message.payload.error_message);
  };

  const handleEdgeAudioFinalized = (_message: EdgeAudioFinalizedMessage) => {
    // La lecture HLS suit la playlist; cet evenement sert a l'observabilite.
  };

  const replayAudioForHistory = async (
    _historyItems: TranslatedNewsItem[],
    historyLabel: string,
  ) => {
    if (
      dailySummaryPlaybackStateRef.current !== "idle" ||
      hlsPlaybackMode === "daily_summary" ||
      hlsPlaybackMode === "calendar_summary"
    ) {
      stopDailySummaryPlayback();
      return;
    }

    if (!isWebPlatform) {
      setLastAudioError("Le résumé audio du jour est disponible sur le client Web uniquement.");
      return;
    }
    if (!authSession) {
      setLastAudioError("Le résumé audio du jour exige une session active.");
      return;
    }

    stopHlsPlayback();
    clearHlsClipQueue();
    setCurrentItem(null);
    setQueue([]);
    setLastAudioError(null);
    recentCompletedAudioIdsRef.current.clear();
    setIsAudioEnabled(true);
    setAudioTransportMode("hls");
    setDailySummaryPlaybackState("waiting");
    const playbackRunId = dailySummaryPlaybackRunIdRef.current + 1;
    dailySummaryPlaybackRunIdRef.current = playbackRunId;
    const abortController = new AbortController();
    dailySummaryAbortControllerRef.current = abortController;

    try {
      const initialSummary = await loadDailySummaryToday({
        dailySummaryTodayUrl,
        authSession,
        signal: abortController.signal,
      });
      const initialSummaryPlaylistUrl = calendarSummaryPlaylistUrl(initialSummary);
      if (initialSummary.status !== "audio_ready" || !initialSummaryPlaylistUrl) {
        const message =
          initialSummary.status === "audio_ready"
            ? "Résumé audio marqué prêt mais aucune playlist HLS n’est disponible."
            : initialSummary.errorMessage ||
              "le résumé du calendrier du jour n’est pas encore prêt";
        setDailySummaryAvailabilityStatus(
          initialSummary.status === "audio_ready" ? "error" : "unavailable",
        );
        setDailySummaryAvailabilityMessage(message);
        throw new Error(message);
      }
      if (
        abortController.signal.aborted ||
        dailySummaryPlaybackRunIdRef.current !== playbackRunId
      ) {
        return;
      }
      await startDailySummaryHlsPlayback(initialSummaryPlaylistUrl);
      setDailySummaryAvailabilityStatus("ready");
      setDailySummaryAvailabilityMessage(null);
      if (
        abortController.signal.aborted ||
        dailySummaryPlaybackRunIdRef.current !== playbackRunId
      ) {
        stopHlsPlayback();
        return;
      }
      setDailySummaryPlaybackState("playing");
    } catch (error) {
      if (
        isAbortError(error) ||
        dailySummaryPlaybackRunIdRef.current !== playbackRunId
      ) {
        return;
      }
      setAudioTransportMode("idle");
      setIsAudioEnabled(false);
      setDailySummaryPlaybackState("idle");
      setLastAudioError(
        error instanceof Error
          ? `Impossible de lancer le résumé IA du jour depuis ${historyLabel}: ${error.message}`
          : `Impossible de lancer le résumé du calendrier du jour depuis ${historyLabel}`,
      );
    } finally {
      if (dailySummaryPlaybackRunIdRef.current === playbackRunId) {
        dailySummaryAbortControllerRef.current = null;
      }
    }
  };

  const changeAudioEnabled = (nextValue: boolean) => {
    if (!isWebPlatform) {
      setLastAudioError("Le flux audio est disponible sur le client Web uniquement.");
      return;
    }

    if (nextValue) {
      abortDailySummaryPolling();
      setDailySummaryPlaybackState("idle");
      stopHlsPlayback();
      clearHlsClipQueue();
      setCurrentItem(null);
      setQueue([]);
      setIsAudioEnabled(true);
      setAudioTransportMode("hls");
      setLastAudioError(null);
      return;
    }

    abortDailySummaryPolling();
    setDailySummaryPlaybackState("idle");
    stopHlsPlayback();
    clearHlsClipQueue();
    setAudioTransportMode("idle");
    setIsAudioEnabled(false);
    setCurrentItem(null);
    setQueue([]);
  };

  const resetAudioForTestRun = () => {
    abortDailySummaryPolling();
    setDailySummaryPlaybackState("idle");
    stopHlsPlayback();
    clearHlsClipQueue();
    setLastAudioError(null);
    currentAudioTimelineItemRef.current = null;
    lastReceivedAudioTimelineItemRef.current = null;
    setCurrentItem(null);
    setQueue([]);
  };

  const resetAudioForReconnect = () => {
    abortDailySummaryPolling();
    setDailySummaryPlaybackState("idle");
    stopHlsPlayback();
    clearHlsClipQueue();
    setCurrentItem(null);
    setQueue([]);
    setLastAudioError(null);
    setAudioTransportMode("idle");
    currentAudioTimelineItemRef.current = null;
    lastReceivedAudioTimelineItemRef.current = null;
  };

  useEffect(() => {
    return () => {
      abortDailySummaryPolling();
      stopHlsPlayback(false);
      clearHlsClipQueue();
    };
  }, []);

  useEffect(() => {
    if (!isAudioEnabled) {
      abortDailySummaryPolling();
      setDailySummaryPlaybackState("idle");
      stopHlsPlayback();
      clearHlsClipQueue();
      setAudioTransportMode("idle");
      setCurrentItem(null);
      setQueue([]);
    }
  }, [isAudioEnabled]);

  useEffect(() => {
    if (dailySummaryPlaybackState !== "playing") {
      return;
    }

    if (
      (hlsPlaybackMode !== "daily_summary" &&
        hlsPlaybackMode !== "calendar_summary") ||
      hlsConnectionState === "error"
    ) {
      setDailySummaryPlaybackState("idle");
    }
  }, [dailySummaryPlaybackState, hlsConnectionState, hlsPlaybackMode]);

  const hasPendingLiveAudioPlayback = () => {
    return (
      currentHlsClipRef.current !== null ||
      queuedHlsClipsRef.current.length > 0 ||
      isHlsPlaybackActive
    );
  };

  const requestVoiceChange = (_voice: string) => {
    setLastAudioError("La voix du direct se règle dans la configuration du serveur ou le profil, pas via le WebSocket audio.");
  };

  const stopAudioPlayback = () => {
    stopDailySummaryPlayback();
  };

  function stopDailySummaryPlayback() {
    dailySummaryPlaybackRunIdRef.current += 1;
    abortDailySummaryPolling();
    stopHlsPlayback();
    clearHlsClipQueue();
    setAudioTransportMode("idle");
    setIsAudioEnabled(false);
    setDailySummaryPlaybackState("idle");
    setLastAudioError(null);
    setCurrentItem(null);
    setQueue([]);
  }

  return {
    audioConnectionState: hlsConnectionState,
    isAudioEnabled,
    isAudioEnabledRef,
    audioTransportMode,
    dailySummaryPlaybackState,
    dailySummaryAvailabilityStatus,
    dailySummaryAvailabilityMessage,
    hlsPlaybackMode,
    hlsReplayTodayMetadata,
    hlsAudioUrl,
    hlsReplayTodayPlaylistUrl,
    lastAudioLaggedCount: null,
    lastAudioError,
    availableVoices: [],
    selectedVoice: null,
    pendingVoiceSelection: null,
    isUpdatingVoice: false,
    audioPlaybackEngine: hlsPlaybackEngine,
    bufferedAudioClipCount: queuedHlsClipCount,
    effectiveAudioVoice: null,
    canChangeVoice: false,
    isHlsPlaybackActive,
    canAdmitAudioTimelineItem,
    recordLastReceivedAudioTimelineItem,
    hasPendingLiveAudioPlayback,
    handleEdgeAudioReady,
    handleCalendarAlertReady,
    handleEdgeAudioFailed,
    handleEdgeAudioFinalized,
    stopAudioPlayback,
    replayAudioForHistory,
    requestVoiceChange,
    changeAudioEnabled,
    resetAudioForReconnect,
    resetAudioForTestRun,
  };
}

function elapsedSinceIsoTimestampMs(timestamp: string) {
  const parsedTimestamp = Date.parse(timestamp);
  if (!Number.isFinite(parsedTimestamp)) {
    return null;
  }

  return Math.max(0, Date.now() - parsedTimestamp);
}

function calendarSummaryPlaylistUrl(summary: {
  currentPlaylistUrl?: string | null;
  playlistUrl?: string | null;
  immutablePlaylistUrl?: string | null;
}) {
  return (
    normalizeAudioUrl(summary.currentPlaylistUrl) ??
    normalizeAudioUrl(summary.playlistUrl) ??
    normalizeAudioUrl(summary.immutablePlaylistUrl)
  );
}

function calendarDaySummaryAvailabilityMessage(status: string) {
  if (status === "audio_generating") {
    return "Résumé audio en génération.";
  }
  if (status === "summarized") {
    return "Résumé audio en préparation.";
  }
  return "Résumé en préparation.";
}

function normalizeAudioUrl(value: string | null | undefined) {
  const text = value?.trim();
  return text ? text : null;
}

function audioReadyIgnoreMessage(reason: AudioReadyIgnoreReason) {
  switch (reason) {
    case "transport_not_hls":
      return "audio_ready reçu mais le transport audio actif n’est pas HLS.";
    case "playback_mode_blocks_live":
      return "audio_ready reçu pendant une lecture du résumé ou d’une rediffusion ; le direct audio est temporairement bloqué.";
    case "missing_playlist_url":
      return "audio_ready reçu sans playlist HLS exploitable.";
    case "audio_disabled":
      return "audio_ready reçu alors que le direct audio est désactivé.";
    case "duplicate_stream":
      return "audio_ready ignoré : flux HLS déjà connu.";
    case "recently_completed":
      return "audio_ready ignoré : clip déjà terminé récemment.";
    case "timeline_watermark":
      return "audio_ready ignoré : événement plus ancien que le watermark audio.";
  }
}

function isAbortError(error: unknown) {
  return (
    typeof error === "object" &&
    error !== null &&
    "name" in error &&
    error.name === "AbortError"
  );
}
