import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type { XRuntimeStatus } from "@shared/api/contracts";
import { loadXRuntimeStatus, updateXRuntimeStatus } from "../services/xRuntimeApi";

type UseXRuntimeOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  wsUrl: string;
};

export function useXRuntime({
  authSession,
  isEnabled,
  wsUrl,
}: UseXRuntimeOptions) {
  const [status, setStatus] = useState<XRuntimeStatus | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!isEnabled || !authSession) {
      setStatus(null);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const nextStatus = await loadXRuntimeStatus({ authSession, wsUrl });
      setStatus(nextStatus);
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, wsUrl]);

  const update = useCallback(
    async (request: {
      enabled?: boolean;
      account_usernames?: string[];
      keywords?: string[];
    }) => {
      if (!authSession) {
        setError("Session admin indisponible.");
        return null;
      }

      setIsUpdating(true);
      setError(null);
      try {
        const nextStatus = await updateXRuntimeStatus({
          authSession,
          request,
          wsUrl,
        });
        setStatus(nextStatus);
        return nextStatus;
      } catch (updateError) {
        setError(errorMessage(updateError));
        return null;
      } finally {
        setIsUpdating(false);
      }
    },
    [authSession, wsUrl],
  );

  const setEnabled = useCallback(
    (enabled: boolean) => update({ enabled }),
    [update],
  );

  const setAccountUsernames = useCallback(
    (accountUsernames: string[]) =>
      update({ account_usernames: accountUsernames }),
    [update],
  );

  const setKeywords = useCallback(
    (keywords: string[]) => update({ keywords }),
    [update],
  );

  useEffect(() => {
    void reload();
  }, [reload]);

  return {
    error,
    isLoading,
    isUpdating,
    reload,
    setAccountUsernames,
    setEnabled,
    setKeywords,
    status,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Erreur X inconnue";
}
