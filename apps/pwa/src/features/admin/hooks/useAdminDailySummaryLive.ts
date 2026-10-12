import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  AdminCalendarDayToday,
  AdminCalendarDayWsMessage,
  ConnectionState,
} from "@shared/api/contracts";
import {
  isWebSocketAuthClose,
  isExpiredWebSocketAuthClose,
  sendWebSocketAuth,
  websocketAuthRejectedMessage,
} from "@shared/api/authTransport";
import { buildAdminDailySummaryWsUrl } from "@shared/api/urls";
import { formatWsCloseMessage } from "@shared/lib/status";
import {
  loadAdminDailySummaryToday,
  runAdminCalendarWeek,
  runAdminDailySummaryToday,
} from "../services/adminDailySummaryApi";

type UseAdminDailySummaryLiveOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  localDay?: string | null;
  wsUrl: string;
};

const activeCalendarPendingRunMaxAgeMs = 45 * 60 * 1000;

export function useAdminDailySummaryLive({
  authSession,
  isEnabled,
  localDay,
  wsUrl,
}: UseAdminDailySummaryLiveOptions) {
  const [snapshot, setSnapshot] = useState<AdminCalendarDayToday | null>(null);
  const [connectionState, setConnectionState] =
    useState<ConnectionState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [lastHeartbeat, setLastHeartbeat] = useState<string | null>(null);
  const [reloadVersion, setReloadVersion] = useState(0);
  const socketRef = useRef<WebSocket | null>(null);

  const closeSocket = useCallback((nextState: ConnectionState = "closed") => {
    try {
      socketRef.current?.close();
    } catch {}
    socketRef.current = null;
    setConnectionState(nextState);
  }, []);

  const reload = useCallback(async () => {
    if (!isEnabled || !authSession) {
      setSnapshot(null);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const nextSnapshot = await loadAdminDailySummaryToday({
        authSession,
        localDay,
        wsUrl,
      });
      setSnapshot(nextSnapshot);
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, localDay, wsUrl]);

  const run = useCallback(async () => {
    if (!isEnabled || !authSession) {
      return;
    }

    setIsRunning(true);
    setError(null);
    try {
      const nextSnapshot = await runAdminDailySummaryToday({
        authSession,
        force: true,
        localDay,
        reason: "manual_admin",
        wsUrl,
      });
      setSnapshot(nextSnapshot);
    } catch (runError) {
      setError(errorMessage(runError));
    } finally {
      setIsRunning(false);
    }
  }, [authSession, isEnabled, localDay, wsUrl]);

  const runWeek = useCallback(async () => {
    if (!isEnabled || !authSession) {
      return;
    }

    setIsRunning(true);
    setError(null);
    try {
      await runAdminCalendarWeek({
        authSession,
        force: true,
        localDay,
        reason: "manual_admin_week",
        wsUrl,
      });
      await reload();
    } catch (runError) {
      setError(errorMessage(runError));
    } finally {
      setIsRunning(false);
    }
  }, [authSession, isEnabled, localDay, reload, wsUrl]);

  const reconnect = useCallback(() => {
    closeSocket("connecting");
    setReloadVersion((version) => version + 1);
  }, [closeSocket]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (!isEnabled || !wsUrl) {
      closeSocket("waiting");
      return;
    }

    if (!authSession) {
      closeSocket("waiting");
      return;
    }

    let cancelled = false;
    let activeSocket: WebSocket | null = null;
    let retry: ReturnType<typeof setTimeout> | undefined;
    let authRecoveryAttempts = 0;
    const connect = async () => {
      try {
        await authSession.validToken();
      } catch {
        if (!cancelled) {
          setError("Authentification temporairement indisponible.");
          if (authSession.isActive)
            retry = setTimeout(() => {
              void connect();
            }, 30_000);
        }
        return;
      }
      if (cancelled) return;
      const socket = new WebSocket(
        buildAdminDailySummaryWsUrl(wsUrl, { localDay }),
      );
      activeSocket = socket;
      let sentToken: string | null = null;
      socketRef.current = socket;
      setConnectionState("connecting");
      setError(null);

      socket.onopen = () => {
        if (cancelled || socketRef.current !== socket) {
          return;
        }
        try {
          sentToken = authSession.getToken();
          sendWebSocketAuth(socket, authSession);
        } catch {
          setConnectionState("error");
          setError("Authentification du flux du calendrier impossible.");
          socket.close();
        }
      };

      socket.onmessage = (event) => {
        if (
          cancelled ||
          socketRef.current !== socket ||
          typeof event.data !== "string"
        ) {
          return;
        }

        try {
          authRecoveryAttempts = 0;
          const message = JSON.parse(event.data) as AdminCalendarDayWsMessage;
          applyMessage(message);
        } catch {
          setConnectionState("error");
          setError("Message WebSocket du calendrier invalide.");
        }
      };

      socket.onerror = () => {
        if (cancelled || socketRef.current !== socket) {
          return;
        }
        setConnectionState("error");
        setError("Connexion au calendrier en direct instable.");
      };

      socket.onclose = (event) => {
        if (cancelled || socketRef.current !== socket) {
          return;
        }
        socketRef.current = null;
        setConnectionState((currentState) =>
          currentState === "error" ? "error" : "closed",
        );
        if (event.code !== 1000 && event.code !== 1005) {
          if (
            isExpiredWebSocketAuthClose(event) &&
            authRecoveryAttempts++ < 1
          ) {
            void authSession
              .recover(sentToken)
              .then(() => {
                if (!cancelled) void connect();
              })
              .catch(() => {
                if (!cancelled) {
                  setError(websocketAuthRejectedMessage);
                  if (authSession.isActive)
                    retry = setTimeout(() => {
                      void connect();
                    }, 30_000);
                }
              });
            return;
          }
          if (isWebSocketAuthClose(event)) {
            setConnectionState("error");
            setError(websocketAuthRejectedMessage);
            return;
          }
          setError(formatWsCloseMessage(event));
        }
      };
    };
    void connect();

    return () => {
      cancelled = true;
      clearTimeout(retry);
      if (socketRef.current === activeSocket) {
        socketRef.current = null;
      }
      activeSocket?.close();
    };
  }, [authSession, closeSocket, isEnabled, localDay, reloadVersion, wsUrl]);

  const applyMessage = useCallback((message: AdminCalendarDayWsMessage) => {
    if (message.type === "hello") {
      setConnectionState("open");
      return;
    }
    if (message.type === "heartbeat") {
      setLastHeartbeat(message.payload.server_time);
      return;
    }
    if (message.type === "snapshot") {
      setSnapshot(message.payload);
      return;
    }
    if (message.type === "run_started" || message.type === "run_updated") {
      setSnapshot((current) =>
        current ? mergeRunIntoSnapshot(current, message.payload) : current,
      );
      return;
    }
    if (message.type === "step") {
      setSnapshot((current) =>
        current
          ? {
              ...current,
              steps: upsertById(current.steps, message.payload),
            }
          : current,
      );
      return;
    }
    if (message.type === "artifact") {
      setSnapshot((current) =>
        current
          ? {
              ...current,
              artifacts: upsertById(current.artifacts, message.payload),
            }
          : current,
      );
      return;
    }
    if (message.type === "summary_updated") {
      setSnapshot((current) =>
        current
          ? {
              ...current,
              summary: message.payload,
            }
          : current,
      );
      return;
    }
    if (message.type === "state_updated") {
      setSnapshot((current) =>
        current
          ? {
              ...current,
              state: message.payload,
            }
          : current,
      );
      return;
    }
    if (message.type === "items_updated") {
      setSnapshot((current) =>
        current
          ? {
              ...current,
              items: mergeById(current.items, message.payload),
            }
          : current,
      );
      return;
    }
    if (message.type === "alert_updated") {
      setSnapshot((current) =>
        current
          ? {
              ...current,
              alerts: upsertById(current.alerts, message.payload),
            }
          : current,
      );
      return;
    }
    if (message.type === "error") {
      setConnectionState("error");
      setError(message.payload.message);
    }
  }, []);

  return {
    connectionState,
    error,
    isLoading,
    isRunning,
    lastHeartbeat,
    reconnect,
    reload,
    run,
    runWeek,
    snapshot,
  };
}

function upsertById<T extends { id: number }>(items: T[], nextItem: T) {
  const nextItems = items.filter((item) => item.id !== nextItem.id);
  nextItems.push(nextItem);
  return nextItems.sort((left, right) => left.id - right.id);
}

function mergeRunIntoSnapshot(
  snapshot: AdminCalendarDayToday,
  run: AdminCalendarDayToday["runs"][number],
): AdminCalendarDayToday {
  const runs = upsertById(snapshot.runs ?? [], run);
  const activeRun = runs.find((item) => isActivePendingRun(item));
  return {
    ...snapshot,
    latestRun: run,
    runs,
    hasActivePendingRun: Boolean(activeRun),
    activePendingRunId: activeRun?.runId ?? null,
  };
}

function isActivePendingRun(run: AdminCalendarDayToday["runs"][number]) {
  if (run.status !== "pending") {
    return false;
  }
  const startedAtMs = new Date(run.startedAt).getTime();
  if (!Number.isFinite(startedAtMs)) {
    return true;
  }
  return Date.now() - startedAtMs < activeCalendarPendingRunMaxAgeMs;
}

function mergeById<T extends { id: number }>(items: T[], nextItems: T[]) {
  return nextItems.reduce((current, item) => upsertById(current, item), items);
}

function errorMessage(error: unknown) {
  return error instanceof Error
    ? error.message
    : "Erreur inconnue du calendrier d’administration.";
}
