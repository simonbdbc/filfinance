import type { AuthSessionClient } from "@shared/api/authSession";
import { useEffect, useRef, useState } from "react";

import type {
  ConnectionState,
  IncomingTraceWsMessage,
  TestRunAccepted,
  TestTraceEvent,
} from "@shared/api/contracts";
import {
  isWebSocketAuthClose,
  isExpiredWebSocketAuthClose,
  sendWebSocketAuth,
  websocketAuthRejectedMessage,
} from "@shared/api/authTransport";
import { MAX_TRACE_ITEMS } from "@shared/config/clientConfig";
import { formatWsCloseMessage } from "@shared/lib/status";

type UseTestRunTraceSocketOptions = {
  traceWsUrl: string | null;
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  onBeforeTrigger?: () => void;
};

export function useTestRunTraceSocket({
  traceWsUrl,
  authSession,
  isEnabled,
  onBeforeTrigger,
}: UseTestRunTraceSocketOptions) {
  const [traceConnectionState, setTraceConnectionState] = useState<ConnectionState>("idle");
  const [isTriggeringTestRun, setIsTriggeringTestRun] = useState(false);
  const [testTraceItems, setTestTraceItems] = useState<TestTraceEvent[]>([]);
  const [activeTestRun, setActiveTestRun] = useState<TestRunAccepted | null>(null);
  const [lastTraceLaggedCount, setLastTraceLaggedCount] = useState<number | null>(null);
  const [lastTraceError, setLastTraceError] = useState<string | null>(null);
  const [traceConnectionVersion, setTraceConnectionVersion] = useState(0);
  const traceReconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const traceSocketRef = useRef<WebSocket | null>(null);
  const activeTestRunIdRef = useRef<string | null>(null);
  const pendingTriggerTestRunRef = useRef(false);
  const onBeforeTriggerRef = useRef(onBeforeTrigger);

  useEffect(() => {
    onBeforeTriggerRef.current = onBeforeTrigger;
  }, [onBeforeTrigger]);

  const sendTriggerTestRun = (socket: WebSocket) => {
    activeTestRunIdRef.current = null;
    pendingTriggerTestRunRef.current = false;
    onBeforeTriggerRef.current?.();
    setActiveTestRun(null);
    setTestTraceItems([]);
    setLastTraceLaggedCount(null);
    setLastTraceError(null);
    setIsTriggeringTestRun(true);

    try {
      socket.send(JSON.stringify({ type: "trigger_test_run" }));
    } catch {
      setIsTriggeringTestRun(false);
      setLastTraceError("Impossible d’envoyer le signal de test au backend");
    }
  };

  const triggerTestRun = () => {
    if (!isEnabled) {
      pendingTriggerTestRunRef.current = false;
      setIsTriggeringTestRun(false);
      setLastTraceError(null);
      return;
    }

    const socket = traceSocketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      pendingTriggerTestRunRef.current = true;
      setIsTriggeringTestRun(true);
      setLastTraceError("Le canal de trace n’est pas connecté. Nouvelle tentative de connexion…");
      setTraceConnectionVersion((currentVersion) => currentVersion + 1);
      return;
    }

    sendTriggerTestRun(socket);
  };

  const resetTraceState = () => {
    activeTestRunIdRef.current = null;
    pendingTriggerTestRunRef.current = false;
    setActiveTestRun(null);
    setTestTraceItems([]);
    setLastTraceLaggedCount(null);
    setLastTraceError(null);
    setIsTriggeringTestRun(false);
  };

  useEffect(() => {
    if (!isEnabled || !traceWsUrl) {
      pendingTriggerTestRunRef.current = false;
      setIsTriggeringTestRun(false);
      setTraceConnectionState("idle");
      setLastTraceError(null);
      return;
    }

    if (!authSession) {
      pendingTriggerTestRunRef.current = false;
      setIsTriggeringTestRun(false);
      setTraceConnectionState("waiting");
      setLastTraceError(null);
      return;
    }

    let cancelled = false;
    let activeSocket: WebSocket | null = null;

    let authRecoveryAttempts = 0;
    const connect = async () => {
      if (cancelled) {
        return;
      }

      setTraceConnectionState("connecting");
      try { await authSession.validToken(); }
      catch {
        if (!cancelled) {
          setLastTraceError("Authentification temporairement indisponible.");
          if (authSession.isActive) traceReconnectTimerRef.current = setTimeout(() => { void connect(); }, 30_000);
        }
        return;
      }
      if (cancelled) return;
      let sentToken: string | null = null;
      const socket = new WebSocket(traceWsUrl);
      activeSocket = socket;
      traceSocketRef.current = socket;

      socket.onopen = () => {
        if (cancelled || traceSocketRef.current !== socket) {
          return;
        }
        try {
          sentToken = authSession.getToken();
          sendWebSocketAuth(socket, authSession);
        } catch {
          setTraceConnectionState("error");
          setLastTraceError("Impossible d’authentifier le canal de trace.");
          socket.close();
          return;
        }
        setTraceConnectionState("open");
        setLastTraceError(null);

        if (pendingTriggerTestRunRef.current) {
          sendTriggerTestRun(socket);
        }
      };

      socket.onmessage = (event) => {
        if (typeof event.data !== "string") {
          return;
        }

        try {
          authRecoveryAttempts = 0;
          const message = JSON.parse(event.data) as IncomingTraceWsMessage;

          if (message.type === "heartbeat" || message.type === "hello") {
            return;
          }

          if (message.type === "test_run_accepted") {
            activeTestRunIdRef.current = message.payload.run_id;
            setActiveTestRun(message.payload);
            setIsTriggeringTestRun(false);
            setTestTraceItems([]);
            setLastTraceLaggedCount(null);
            setLastTraceError(null);
            return;
          }

          if (message.type === "test_trace") {
            const activeRunId = activeTestRunIdRef.current;
            if (activeRunId && message.payload.run_id !== activeRunId) {
              return;
            }

            setTestTraceItems((previousItems) =>
              [...previousItems, message.payload].slice(-MAX_TRACE_ITEMS)
            );
            if (message.payload.status === "error") {
              setLastTraceError(message.payload.message);
              setIsTriggeringTestRun(false);
            }
            return;
          }

          if (message.type === "lagged") {
            setLastTraceLaggedCount(message.payload.skipped);
            return;
          }

          if (message.type === "error") {
            setIsTriggeringTestRun(false);
            setLastTraceError(message.payload.message);
          }
        } catch {
          setLastTraceError("Message WebSocket de trace invalide");
          setIsTriggeringTestRun(false);
        }
      };

      socket.onerror = () => {
        if (cancelled || traceSocketRef.current !== socket) {
          return;
        }
        setTraceConnectionState("error");
        setLastTraceError("Erreur réseau sur le canal de trace");
      };

      socket.onclose = (event) => {
        if (cancelled) {
          return;
        }

        if (traceSocketRef.current === socket) {
          traceSocketRef.current = null;
        }
        setTraceConnectionState("closed");

        if (isExpiredWebSocketAuthClose(event) && authRecoveryAttempts++ < 1) {
          void authSession.recover(sentToken).then(() => { if (!cancelled) void connect(); }).catch(() => {
            if (!cancelled) {
              setTraceConnectionState("error"); setLastTraceError(websocketAuthRejectedMessage);
              if (authSession.isActive) traceReconnectTimerRef.current = setTimeout(() => { void connect(); }, 30_000);
            }
          });
          return;
        }
        if (isWebSocketAuthClose(event)) {
          setTraceConnectionState("error");
          setLastTraceError(websocketAuthRejectedMessage);
          setIsTriggeringTestRun(false);
          return;
        }

        setLastTraceError(formatWsCloseMessage(event));
        traceReconnectTimerRef.current = setTimeout(connect, 1500);
      };
    };

    connect();

    return () => {
      cancelled = true;
      if (traceReconnectTimerRef.current) {
        clearTimeout(traceReconnectTimerRef.current);
        traceReconnectTimerRef.current = null;
      }
      if (traceSocketRef.current === activeSocket) {
        traceSocketRef.current = null;
      }
      activeSocket?.close();
    };
  }, [authSession, isEnabled, traceConnectionVersion, traceWsUrl]);

  return {
    traceConnectionState,
    isTriggeringTestRun,
    testTraceItems,
    activeTestRun,
    lastTraceLaggedCount,
    lastTraceError,
    triggerTestRun,
    resetTraceState,
  };
}
