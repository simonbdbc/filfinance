import type { AuthSessionClient } from "@shared/api/authSession";
import { useEffect, useRef, useState } from "react";

import type { TranslatedNewsItem } from "@shared/api/contracts";
import {
  type DailyNewsHistory,
  loadDailyNewsHistory,
} from "../services/loadDailyNewsHistory";

type UseDailyNewsHistoryOptions = {
  wsUrl: string;
  authSession: AuthSessionClient | null;
  currentLocalDayKey: string;
  previousLocalDayKey: string;
  reloadVersion: number;
  onLoaded?: (history: DailyNewsHistory) => void;
};

export function useDailyNewsHistory({
  wsUrl,
  authSession,
  currentLocalDayKey,
  previousLocalDayKey,
  reloadVersion,
  onLoaded,
}: UseDailyNewsHistoryOptions) {
  const [dailyFeedHistory, setDailyFeedHistory] = useState<TranslatedNewsItem[]>([]);
  const [dailyTestRunHistory, setDailyTestRunHistory] = useState<TranslatedNewsItem[]>([]);
  const [isDailyHistoryLoading, setIsDailyHistoryLoading] = useState(false);
  const [dailyHistoryError, setDailyHistoryError] = useState<string | null>(null);
  const onLoadedRef = useRef(onLoaded);

  useEffect(() => {
    onLoadedRef.current = onLoaded;
  }, [onLoaded]);

  useEffect(() => {
    let cancelled = false;
    const abortController = new AbortController();

    const loadHistory = async () => {
      setIsDailyHistoryLoading(true);
      setDailyHistoryError(null);

      try {
        const history = await loadDailyNewsHistory({
          wsUrl,
          authSession,
          currentLocalDayKey,
          previousLocalDayKey,
          signal: abortController.signal,
        });

        if (cancelled) {
          return;
        }

        setDailyFeedHistory(history.feedItems);
        setDailyTestRunHistory(history.testRunItems);
        onLoadedRef.current?.(history);
        setDailyHistoryError(null);
      } catch (error) {
        if (cancelled || abortController.signal.aborted) {
          return;
        }

        setDailyHistoryError(
          error instanceof Error
            ? `Impossible de charger l’historique journalier : ${error.message}`
            : "Impossible de charger l’historique journalier"
        );
      } finally {
        if (!cancelled) {
          setIsDailyHistoryLoading(false);
        }
      }
    };

    void loadHistory();

    return () => {
      cancelled = true;
      abortController.abort();
    };
  }, [authSession, currentLocalDayKey, previousLocalDayKey, reloadVersion, wsUrl]);

  return {
    dailyFeedHistory,
    setDailyFeedHistory,
    dailyTestRunHistory,
    setDailyTestRunHistory,
    isDailyHistoryLoading,
    dailyHistoryError,
    setDailyHistoryError,
  };
}
