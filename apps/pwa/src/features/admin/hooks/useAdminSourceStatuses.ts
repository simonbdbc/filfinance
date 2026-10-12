import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type { SourceStatus } from "@shared/api/contracts";
import { loadAdminSourceStatuses } from "../services/sourceStatusesApi";

type UseAdminSourceStatusesOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  wsUrl: string;
};

export function useAdminSourceStatuses({
  authSession,
  isEnabled,
  wsUrl,
}: UseAdminSourceStatusesOptions) {
  const [statuses, setStatuses] = useState<SourceStatus[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [reloadVersion, setReloadVersion] = useState(0);

  const reload = useCallback(() => {
    setReloadVersion((currentVersion) => currentVersion + 1);
  }, []);

  useEffect(() => {
    if (!isEnabled || !authSession) {
      setStatuses([]);
      setError(null);
      setIsLoading(false);
      return;
    }

    const controller = new AbortController();
    setIsLoading(true);
    setError(null);

    loadAdminSourceStatuses({
      authSession,
      signal: controller.signal,
      wsUrl,
    })
      .then((nextStatuses) => {
        setStatuses(nextStatuses);
      })
      .catch((nextError: unknown) => {
        if (controller.signal.aborted) {
          return;
        }

        setStatuses([]);
        setError(
          nextError instanceof Error
            ? nextError.message
            : "Impossible de charger les statuts admin des sources.",
        );
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoading(false);
        }
      });

    return () => controller.abort();
  }, [authSession, isEnabled, reloadVersion, wsUrl]);

  return {
    error,
    isLoading,
    reload,
    statuses,
  };
}
