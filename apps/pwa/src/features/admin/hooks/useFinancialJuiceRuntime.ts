import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type {
  FinancialJuiceBrowserRuntimeStatus,
  FinancialJuiceRuntimeStatus,
} from "@shared/api/contracts";
import {
  loadFinancialJuiceBrowserRuntimeStatus,
  loadFinancialJuiceRuntimeStatus,
  updateFinancialJuiceRuntimeStatus,
} from "../services/financialJuiceRuntimeApi";

type UseFinancialJuiceRuntimeOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  wsUrl: string;
};

export function useFinancialJuiceRuntime({
  authSession,
  isEnabled,
  wsUrl,
}: UseFinancialJuiceRuntimeOptions) {
  const [status, setStatus] = useState<FinancialJuiceRuntimeStatus | null>(null);
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
      const nextStatus = await loadFinancialJuiceRuntimeStatus({
        authSession,
        wsUrl,
      });
      setStatus(nextStatus);
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, wsUrl]);

  const setEnabled = useCallback(
    async (enabled: boolean) => {
      if (!authSession) {
        setError("Session admin indisponible.");
        return;
      }

      setIsUpdating(true);
      setError(null);
      try {
        const nextStatus = await updateFinancialJuiceRuntimeStatus({
          authSession,
          enabled,
          wsUrl,
        });
        setStatus(nextStatus);
      } catch (updateError) {
        setError(errorMessage(updateError));
      } finally {
        setIsUpdating(false);
      }
    },
    [authSession, wsUrl],
  );

  useEffect(() => {
    void reload();
  }, [reload]);

  return {
    error,
    isLoading,
    isUpdating,
    reload,
    setEnabled,
    status,
  };
}

export function useFinancialJuiceBrowserRuntime({
  authSession,
  isEnabled,
  wsUrl,
}: UseFinancialJuiceRuntimeOptions) {
  const [status, setStatus] = useState<FinancialJuiceBrowserRuntimeStatus | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!isEnabled || !authSession) {
      setStatus(null);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const nextStatus = await loadFinancialJuiceBrowserRuntimeStatus({
        authSession,
        wsUrl,
      });
      setStatus(nextStatus);
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, wsUrl]);

  useEffect(() => {
    void reload();
  }, [reload]);

  return {
    error,
    isLoading,
    reload,
    status,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Erreur FinancialJuice inconnue";
}
