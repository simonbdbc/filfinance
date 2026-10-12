import type { AuthSessionClient } from "@shared/api/authSession";
import { useEffect, useRef, useState } from "react";

import {
  appendEdgeLiveTicket,
  loadEdgeLiveTicket,
} from "../services/edgeLiveTicketApi";
import type {
  ConnectionState,
  EdgeAudioState,
  EdgeAudioEvent,
  EdgeAudioFailedMessage,
  EdgeAudioFinalizedMessage,
  EdgeAudioReadyMessage,
  EdgeCalendarAlertReadyMessage,
  EdgeTranslatedNewsItemMessage,
  EdgeNowPlaying,
  IncomingLiveMetadataMessage,
  PublicCalendarDaySummaryStatus,
  PublicSourceStatus,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import {
  isWebSocketAuthClose,
  sendWebSocketAuth,
  websocketAuthRejectedMessage,
} from "@shared/api/authTransport";
import {
  formatWsCloseMessage,
  WS_ERROR_DELAY_MS,
  WS_ERROR_MESSAGE,
} from "@shared/lib/status";

type UseNewsSocketOptions = {
  wsUrl: string;
  fallbackWsUrl?: string | null;
  edgeTicketUrl?: string | null;
  authSession: AuthSessionClient | null;
  reloadVersion: number;
  onTranslatedNewsItem: (item: TranslatedNewsItem) => void;
  onNowPlaying?: (nowPlaying: EdgeNowPlaying | null) => void;
  onAudioState?: (audioState: EdgeAudioState | null) => void;
  onAudioReady?: (message: EdgeAudioReadyMessage) => void;
  onAudioFailed?: (message: EdgeAudioFailedMessage) => void;
  onAudioFinalized?: (message: EdgeAudioFinalizedMessage) => void;
  onCalendarAlertReady?: (message: EdgeCalendarAlertReadyMessage) => void;
  onCalendarDaySummary?: (summary: PublicCalendarDaySummaryStatus) => void;
  onSourceStatuses?: (sources: PublicSourceStatus[]) => void;
};

const EDGE_FALLBACK_MESSAGE = "Flux Edge indisponible, connexion de repli au serveur active.";

function isRecoverableTransportError(message: string | null): message is string {
  return (
    message === WS_ERROR_MESSAGE ||
    message === EDGE_FALLBACK_MESSAGE ||
    Boolean(message?.startsWith("WebSocket fermé"))
  );
}

function isRefreshableWebSocketAuthClose(event: CloseEvent) {
  const reason = event.reason.toLowerCase();
  return event.code === 1008 && reason.includes("token expired");
}

export function useNewsSocket({
  wsUrl,
  fallbackWsUrl = null,
  edgeTicketUrl = null,
  authSession,
  reloadVersion,
  onTranslatedNewsItem,
  onNowPlaying,
  onAudioState,
  onAudioReady,
  onAudioFailed,
  onAudioFinalized,
  onCalendarAlertReady,
  onCalendarDaySummary,
  onSourceStatuses,
}: UseNewsSocketOptions) {
  const [connectionState, setConnectionState] =
    useState<ConnectionState>("idle");
  const [lastLaggedCount, setLastLaggedCount] = useState<number | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const [activeWsUrl, setActiveWsUrl] = useState(wsUrl);
  const [isUsingFallback, setIsUsingFallback] = useState(false);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const wsErrorTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptCountRef = useRef(0);
  const authRecoveryAttemptCountRef = useRef(0);
  const seenItemIdsRef = useRef<Set<number>>(new Set());
  const socketRef = useRef<WebSocket | null>(null);
  const onTranslatedNewsItemRef = useRef(onTranslatedNewsItem);
  const onNowPlayingRef = useRef(onNowPlaying);
  const onAudioStateRef = useRef(onAudioState);
  const onAudioReadyRef = useRef(onAudioReady);
  const onAudioFailedRef = useRef(onAudioFailed);
  const onAudioFinalizedRef = useRef(onAudioFinalized);
  const onCalendarAlertReadyRef = useRef(onCalendarAlertReady);
  const onCalendarDaySummaryRef = useRef(onCalendarDaySummary);
  const onSourceStatusesRef = useRef(onSourceStatuses);

  useEffect(() => {
    onTranslatedNewsItemRef.current = onTranslatedNewsItem;
  }, [onTranslatedNewsItem]);

  useEffect(() => {
    onNowPlayingRef.current = onNowPlaying;
  }, [onNowPlaying]);

  useEffect(() => {
    onAudioStateRef.current = onAudioState;
  }, [onAudioState]);

  useEffect(() => {
    onAudioReadyRef.current = onAudioReady;
  }, [onAudioReady]);

  useEffect(() => {
    onAudioFailedRef.current = onAudioFailed;
  }, [onAudioFailed]);

  useEffect(() => {
    onAudioFinalizedRef.current = onAudioFinalized;
  }, [onAudioFinalized]);

  useEffect(() => {
    onCalendarAlertReadyRef.current = onCalendarAlertReady;
  }, [onCalendarAlertReady]);

  useEffect(() => {
    onCalendarDaySummaryRef.current = onCalendarDaySummary;
  }, [onCalendarDaySummary]);

  useEffect(() => {
    onSourceStatusesRef.current = onSourceStatuses;
  }, [onSourceStatuses]);

  const clearWsErrorTimer = () => {
    if (wsErrorTimerRef.current) {
      clearTimeout(wsErrorTimerRef.current);
      wsErrorTimerRef.current = null;
    }
  };

  const scheduleWsError = (message = WS_ERROR_MESSAGE) => {
    clearWsErrorTimer();
    wsErrorTimerRef.current = setTimeout(() => {
      setLastError((currentError) =>
        currentError === null || isRecoverableTransportError(currentError)
          ? message
          : currentError,
      );
    }, WS_ERROR_DELAY_MS);
  };

  const resetNewsSocketState = () => {
    clearWsErrorTimer();
    reconnectAttemptCountRef.current = 0;
    authRecoveryAttemptCountRef.current = 0;
    seenItemIdsRef.current.clear();
    setActiveWsUrl(wsUrl);
    setIsUsingFallback(false);
    setLastLaggedCount(null);
    setLastError(null);
  };

  useEffect(() => {
    if (!wsUrl) {
      setConnectionState("idle");
      setActiveWsUrl("");
      setIsUsingFallback(false);
      return;
    }

    if (!authSession) {
      setConnectionState("waiting");
      setActiveWsUrl(wsUrl);
      setIsUsingFallback(false);
      setLastError(null);
      return;
    }

    let cancelled = false;
    let activeSocket: WebSocket | null = null;
    authRecoveryAttemptCountRef.current = 0;

    const connect = async (
      targetWsUrl = wsUrl,
      usingFallback = false,
    ) => {
      if (cancelled) {
        return;
      }

      setConnectionState("connecting");
      setActiveWsUrl(targetWsUrl);
      setIsUsingFallback(usingFallback);
      let connectWsUrl = targetWsUrl;

      if (!usingFallback && edgeTicketUrl) {
        try {
          const ticket = await loadEdgeLiveTicket({
            ticketUrl: edgeTicketUrl,
            authSession,
          });
          connectWsUrl = appendEdgeLiveTicket(targetWsUrl, ticket.ticket);
        } catch {
          if (fallbackWsUrl) {
            clearWsErrorTimer();
            setLastError(EDGE_FALLBACK_MESSAGE);
            reconnectTimerRef.current = setTimeout(() => {
              void connect(fallbackWsUrl.trim(), true);
            }, 1500);
            return;
          }

          setConnectionState("error");
          setLastError("Impossible de générer le ticket du flux Edge en direct.");
          return;
        }
      }

      if (cancelled) {
        return;
      }

      try { await authSession.validToken(); }
      catch {
        if (!cancelled) {
          setConnectionState("error");
          setLastError("Authentification temporairement indisponible.");
          if (authSession.isActive) reconnectTimerRef.current = setTimeout(() => { void connect(targetWsUrl, usingFallback); }, 30_000);
        }
        return;
      }
      if (cancelled) return;
      let sentToken: string | null = null;
      const socket = new WebSocket(connectWsUrl);
      activeSocket = socket;
      socketRef.current = socket;

      socket.onopen = () => {
        if (cancelled || socketRef.current !== socket) {
          return;
        }
        try {
          sentToken = authSession.getToken();
          sendWebSocketAuth(socket, authSession);
        } catch {
          setConnectionState("error");
          setLastError("Impossible d’authentifier le flux d’actualités.");
          socket.close();
          return;
        }
        clearWsErrorTimer();
        reconnectAttemptCountRef.current = 0;
        setConnectionState("open");
        // Une reconnexion reussie, y compris via le fallback backend, transforme
        // l'alerte edge en etat recupere; les erreurs metier restent visibles.
        setLastError((currentError) =>
          isRecoverableTransportError(currentError) ? null : currentError,
        );
      };

      socket.onmessage = (event) => {
        if (typeof event.data !== "string") {
          return;
        }

        try {
          const message = JSON.parse(event.data) as IncomingLiveMetadataMessage;
          if (message.type === "hello" || message.type === "heartbeat") authRecoveryAttemptCountRef.current = 0;

          if (isLoadTestLiveMessage(message)) {
            return;
          }

          if (message.type === "heartbeat") {
            dispatchSourceStatuses(message.payload.sources);
            dispatchCalendarDaySummary(message.payload.calendar_day_summary);
            return;
          }

          if (message.type === "hello") {
            if ("snapshot" in message.payload) {
              const snapshot = message.payload.snapshot;
              if (!isLoadTestEventId(snapshot.latest_event_id)) {
                dispatchSourceStatuses(snapshot.source_statuses);
                dispatchCalendarDaySummary(snapshot.calendar_day_summary);
                onNowPlayingRef.current?.(snapshot.now_playing);
                onAudioStateRef.current?.(snapshot.audio_state);
              }
              for (const translatedMessage of message.payload.snapshot
                .translated_news_items ?? []) {
                if (isLoadTestLiveMessage(translatedMessage)) {
                  continue;
                }
                dispatchTranslatedNewsItem(translatedMessage);
              }
              for (const audioEvent of message.payload.snapshot.audio_events ?? []) {
                if (isLoadTestLiveMessage(audioEvent)) {
                  continue;
                }
                dispatchAudioEvent(audioEvent);
              }
            } else {
              dispatchSourceStatuses(message.payload.sources);
              dispatchCalendarDaySummary(message.payload.calendar_day_summary);
            }
            return;
          }

          if (message.type === "now_playing") {
            onNowPlayingRef.current?.(message.payload.now_playing);
            return;
          }

          if (message.type === "audio_state") {
            onAudioStateRef.current?.(message.payload.state);
            return;
          }

          if (
            message.type === "audio_ready" ||
            message.type === "audio_failed" ||
            message.type === "audio_finalized" ||
            message.type === "calendar_alert_ready"
          ) {
            dispatchAudioEvent(message);
            return;
          }

          if (message.type === "translated_news_item") {
            dispatchTranslatedNewsItem(message);
            return;
          }

          if (message.type === "source_status") {
            dispatchSourceStatuses(message.payload.sources);
            dispatchCalendarDaySummary(message.payload.calendar_day_summary);
            return;
          }

          if (message.type === "lagged") {
            setLastLaggedCount(message.payload.skipped);
            return;
          }

          if (
            message.type === "system_notice" &&
            message.payload.severity !== "info"
          ) {
            setLastError(
              message.payload.detail
                ? `${message.payload.message}: ${message.payload.detail}`
                : message.payload.message,
            );
          }
        } catch {
          setLastError("Message WebSocket invalide");
        }
      };

      socket.onerror = () => {
        if (cancelled || socketRef.current !== socket) {
          return;
        }
        setConnectionState("error");
        scheduleWsError();
      };

      socket.onclose = (event) => {
        if (cancelled) {
          return;
        }

        if (socketRef.current === socket) {
          socketRef.current = null;
        }
        reconnectAttemptCountRef.current += 1;
        setConnectionState("closed");

        if (isWebSocketAuthClose(event)) {
          clearWsErrorTimer();
          const canRefreshAuth =
            isRefreshableWebSocketAuthClose(event) &&
            authRecoveryAttemptCountRef.current < 1;

          if (!canRefreshAuth) {
            setConnectionState("error");
            setLastError(websocketAuthRejectedMessage);
            return;
          }

          authRecoveryAttemptCountRef.current += 1;
          setConnectionState("waiting");
          setLastError(null);
          void authSession.recover(sentToken)
            .then((refreshedToken) => {
              if (cancelled) {
                return;
              }

              if (!refreshedToken) {
                setConnectionState("error");
                setLastError(websocketAuthRejectedMessage);
                return;
              }

              // Reconnecter le transport sans reinitialiser le runtime.
              reconnectTimerRef.current = setTimeout(() => {
                void connect(targetWsUrl, usingFallback);
              }, 250);
            })
            .catch(() => {
              if (!cancelled) {
                setConnectionState("error");
                setLastError(websocketAuthRejectedMessage);
                if (authSession.isActive) reconnectTimerRef.current = setTimeout(() => { void connect(targetWsUrl, usingFallback); }, 30_000);
              }
            });
          return;
        }

        const shouldUseFallback =
          !usingFallback &&
          typeof fallbackWsUrl === "string" &&
          fallbackWsUrl.trim().length > 0;
        const nextWsUrl = shouldUseFallback
          ? fallbackWsUrl.trim()
          : targetWsUrl;
        const nextUsesFallback = usingFallback || shouldUseFallback;

        if (shouldUseFallback) {
          clearWsErrorTimer();
          setLastError(EDGE_FALLBACK_MESSAGE);
        } else {
          scheduleWsError(formatWsCloseMessage(event));
        }
        reconnectTimerRef.current = setTimeout(() => {
          void connect(nextWsUrl, nextUsesFallback);
        }, 1500);
      };
    };

    void connect();

    return () => {
      cancelled = true;
      if (reconnectTimerRef.current) {
        clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = null;
      }
      clearWsErrorTimer();
      if (socketRef.current === activeSocket) {
        socketRef.current = null;
      }
      activeSocket?.close();
    };
  }, [
    authSession,
    edgeTicketUrl,
    fallbackWsUrl,
      reloadVersion,
    wsUrl,
  ]);

  useEffect(() => {
    return () => {
      clearWsErrorTimer();
      socketRef.current?.close();
    };
  }, []);

  const dispatchAudioEvent = (message: EdgeAudioEvent) => {
    if (message.type === "audio_ready") {
      onAudioReadyRef.current?.(message);
      return;
    }

    if (message.type === "audio_failed") {
      onAudioFailedRef.current?.(message);
      return;
    }

    if (message.type === "calendar_alert_ready") {
      onCalendarAlertReadyRef.current?.(message);
      return;
    }

    onAudioFinalizedRef.current?.(message);
  };

  const dispatchSourceStatuses = (
    sources: PublicSourceStatus[] | null | undefined,
  ) => {
    if (Array.isArray(sources)) {
      onSourceStatusesRef.current?.(sources);
    }
  };

  const dispatchCalendarDaySummary = (
    summary: PublicCalendarDaySummaryStatus | null | undefined,
  ) => {
    if (summary) {
      onCalendarDaySummaryRef.current?.(summary);
    }
  };

  const dispatchTranslatedNewsItem = (
    message: IncomingLiveMetadataMessage | EdgeTranslatedNewsItemMessage,
  ) => {
    const item = translatedItemFromLiveMessage(message);
    if (!item || item.language !== "fr" || item.status !== "ready") {
      return;
    }

    if (seenItemIdsRef.current.has(item.id)) {
      return;
    }

    seenItemIdsRef.current.add(item.id);
    setLastLaggedCount(null);
    onTranslatedNewsItemRef.current(item);
  };

  return {
    connectionState,
    activeWsUrl,
    isUsingFallback,
    lastLaggedCount,
    lastError,
    resetNewsSocketState,
  };
}

function translatedItemFromLiveMessage(
  message: IncomingLiveMetadataMessage,
): TranslatedNewsItem | null {
  if (message.type !== "translated_news_item") {
    return null;
  }

  if ("item" in message.payload) {
    return message.payload.item;
  }

  return message.payload;
}

const LOAD_TEST_EVENT_ID_PREFIX = "load-test:";

function isLoadTestLiveMessage(
  message: IncomingLiveMetadataMessage | EdgeAudioEvent | EdgeTranslatedNewsItemMessage,
) {
  return isLoadTestEventId(liveEventIdFromMessage(message));
}

function liveEventIdFromMessage(
  message: IncomingLiveMetadataMessage | EdgeAudioEvent | EdgeTranslatedNewsItemMessage,
) {
  const payload = message.payload;
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const eventId = (payload as { event_id?: unknown }).event_id;
  return typeof eventId === "string" ? eventId : null;
}

function isLoadTestEventId(eventId: string | null | undefined) {
  return typeof eventId === "string" && eventId.startsWith(LOAD_TEST_EVENT_ID_PREFIX);
}
