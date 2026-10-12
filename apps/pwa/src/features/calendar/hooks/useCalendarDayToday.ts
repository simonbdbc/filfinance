import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type { CalendarDayToday } from "@shared/api/contracts";
import { loadCalendarDayToday } from "../services/calendarDayApi";

type UseCalendarDayTodayOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  localDay?: string | null;
  wsUrl: string;
};

export function useCalendarDayToday({
  authSession,
  isEnabled,
  localDay,
  wsUrl,
}: UseCalendarDayTodayOptions) {
  const [data, setData] = useState<CalendarDayToday | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const reload = useCallback(
    async (signal?: AbortSignal) => {
      if (!isEnabled || !authSession) {
        setData(null);
        setError(null);
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      setError(null);
      try {
        const payload = await loadCalendarDayToday({
          authSession,
          localDay,
          signal,
          wsUrl,
        });
        setData(payload);
      } catch (loadError) {
        if (isAbortError(loadError)) {
          return;
        }
        setError(
          loadError instanceof Error
            ? loadError.message
            : "Impossible de charger le calendrier.",
        );
      } finally {
        if (!signal?.aborted) {
          setIsLoading(false);
        }
      }
    },
    [authSession, isEnabled, localDay, wsUrl],
  );

  useEffect(() => {
    const controller = new AbortController();
    void reload(controller.signal);
    return () => controller.abort();
  }, [reload]);

  return {
    data,
    error,
    isLoading,
    reload: () => {
      void reload();
    },
  };
}

function isAbortError(error: unknown) {
  return error instanceof Error && error.name === "AbortError";
}
