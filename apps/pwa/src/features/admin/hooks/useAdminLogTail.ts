import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  AdminLogLine,
  ConnectionState,
  IncomingAdminLogWsMessage,
} from "@shared/api/contracts";
import {
  isWebSocketAuthClose,
  isExpiredWebSocketAuthClose,
  sendWebSocketAuth,
  websocketAuthRejectedMessage,
} from "@shared/api/authTransport";
import { buildAdminLogsTailWsUrl } from "@shared/api/urls";
import { formatWsCloseMessage } from "@shared/lib/status";

type UseAdminLogTailOptions = {
  authSession: AuthSessionClient | null;
  day: string | null;
  isEnabled: boolean;
  level: string | null;
  query: string;
  wsUrl: string;
};

const maxDisplayedLogLines = 3000;
const initialTailLimit = 500;

export function useAdminLogTail({
  authSession,
  day,
  isEnabled,
  level,
  query,
  wsUrl,
}: UseAdminLogTailOptions) {
  const [items, setItems] = useState<AdminLogLine[]>([]);
  const [connectionState, setConnectionState] =
    useState<ConnectionState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [isDirectEnabled, setIsDirectEnabled] = useState(false);
  const [lastHeartbeat, setLastHeartbeat] = useState<string | null>(null);
  const [lastFileSizeBytes, setLastFileSizeBytes] = useState<number | null>(
    null,
  );
  const [reloadVersion, setReloadVersion] = useState(0);
  const socketRef = useRef<WebSocket | null>(null);

  const closeSocket = useCallback((nextState: ConnectionState = "closed") => {
    try {
      socketRef.current?.close();
    } catch {}
    socketRef.current = null;
    setConnectionState(nextState);
  }, []);

  const startDirect = useCallback(() => {
    setIsDirectEnabled(true);
  }, []);

  const pauseDirect = useCallback(() => {
    setIsDirectEnabled(false);
    closeSocket("closed");
  }, [closeSocket]);

  const resumeDirect = useCallback(() => {
    setIsDirectEnabled(true);
    setReloadVersion((currentVersion) => currentVersion + 1);
  }, []);

  const reconnect = useCallback(() => {
    closeSocket("connecting");
    setIsDirectEnabled(true);
    setReloadVersion((currentVersion) => currentVersion + 1);
  }, [closeSocket]);

  const clear = useCallback(() => {
    setItems([]);
  }, []);

  useEffect(() => {
    if (!isEnabled || !day || !wsUrl) {
      closeSocket("waiting");
      return;
    }

    if (!isDirectEnabled) {
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
        buildAdminLogsTailWsUrl(wsUrl, {
          day,
          level,
          limit: initialTailLimit,
          query: query.trim() || null,
        }),
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
          setError("Authentification du flux de journaux impossible.");
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
          const message = JSON.parse(event.data) as IncomingAdminLogWsMessage;
          if (message.type === "hello") {
            setConnectionState("open");
            return;
          }
          if (message.type === "heartbeat") {
            setLastHeartbeat(message.payload.server_time);
            return;
          }
          if (message.type === "snapshot") {
            setItems(limitLogLines(message.payload.items));
            setLastFileSizeBytes(message.payload.file_size_bytes);
            return;
          }
          if (message.type === "append") {
            setItems((currentItems) =>
              limitLogLines([...currentItems, ...message.payload.items]),
            );
            setLastFileSizeBytes(message.payload.file_size_bytes);
            return;
          }
          if (message.type === "error") {
            setConnectionState("error");
            setError(message.payload.message);
          }
        } catch {
          setConnectionState("error");
          setError("Message WebSocket des journaux invalide.");
        }
      };

      socket.onerror = () => {
        if (cancelled || socketRef.current !== socket) {
          return;
        }
        setConnectionState("error");
        setError("Connexion aux journaux en direct instable.");
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
  }, [
    authSession,
    closeSocket,
    day,
    isDirectEnabled,
    isEnabled,
    level,
    query,
    reloadVersion,
    wsUrl,
  ]);

  return {
    clear,
    connectionState,
    error,
    isDirectEnabled,
    items,
    lastFileSizeBytes,
    lastHeartbeat,
    pauseDirect,
    reconnect,
    resumeDirect,
    startDirect,
  };
}

function limitLogLines(items: AdminLogLine[]) {
  if (items.length <= maxDisplayedLogLines) {
    return items;
  }

  return items.slice(items.length - maxDisplayedLogLines);
}
