import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useState } from "react";

import type { AdminLogDay } from "@shared/api/contracts";
import { loadAdminLogDays } from "../services/adminLogsApi";

type UseAdminLogsOptions = {
  authSession: AuthSessionClient | null;
  isEnabled: boolean;
  wsUrl: string;
};

export function useAdminLogs({
  authSession,
  isEnabled,
  wsUrl,
}: UseAdminLogsOptions) {
  const [days, setDays] = useState<AdminLogDay[]>([]);
  const [selectedDay, setSelectedDay] = useState<string | null>(null);
  const [level, setLevel] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!isEnabled || !authSession) {
      setDays([]);
      setSelectedDay(null);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const daysPayload = await loadAdminLogDays({ authSession, wsUrl });
      const nextDays = daysPayload.days;
      const nextSelectedDay =
        selectedDay && nextDays.some((day) => day.day === selectedDay)
          ? selectedDay
          : nextDays[0]?.day ?? null;

      setDays(nextDays);
      setSelectedDay(nextSelectedDay);
    } catch (loadError) {
      setError(errorMessage(loadError));
    } finally {
      setIsLoading(false);
    }
  }, [authSession, isEnabled, selectedDay, wsUrl]);

  useEffect(() => {
    void reload();
  }, [reload]);

  return {
    days,
    error,
    isLoading,
    level,
    query,
    reload,
    selectedDay,
    setLevel,
    setQuery,
    setSelectedDay,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Erreur inconnue lors de la lecture des journaux.";
}
