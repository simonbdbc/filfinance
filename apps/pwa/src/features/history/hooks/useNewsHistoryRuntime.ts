import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  PublicCalendarDaySummaryStatus,
  PublicSourceStatus,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import { MAX_HISTORY_ITEMS } from "@shared/config/clientConfig";
import {
  dedupeRealFeedItemsBySourcePriority,
  isDisplayableRealFeedItem,
  itemTimestampValue,
  localDayKeyDaysAgo,
  localDayKeyFromDate,
  localDayKeyFromTimestamp,
  mergeHistoryItemsNewestFirst,
  pickNewestKnownItem,
  sortHistoryNewestFirst,
} from "@shared/lib/newsHistory";
import { useDailyNewsHistory } from "./useDailyNewsHistory";

type UseNewsHistoryRuntimeOptions = {
  authSession: AuthSessionClient | null;
  reloadVersion: number;
  wsUrl: string;
};

const realFeedSources = new Set(["financial_juice", "financial_juice_browser"]);

export function useNewsHistoryRuntime({
  authSession,
  reloadVersion,
  wsUrl,
}: UseNewsHistoryRuntimeOptions) {
  const [history, setHistory] = useState<TranslatedNewsItem[]>([]);
  const [newsSourceStatuses, setNewsSourceStatuses] = useState<
    PublicSourceStatus[]
  >([]);
  const [calendarDaySummaryStatus, setCalendarDaySummaryStatus] =
    useState<PublicCalendarDaySummaryStatus | null>(null);
  const [lastItem, setLastItem] = useState<TranslatedNewsItem | null>(null);
  const [isPreviousFeedDayVisible, setIsPreviousFeedDayVisible] =
    useState(false);
  const historyRef = useRef<TranslatedNewsItem[]>([]);
  const dailyFeedHistoryRef = useRef<TranslatedNewsItem[]>([]);
  const dailyTestRunHistoryRef = useRef<TranslatedNewsItem[]>([]);
  const lastItemRef = useRef<TranslatedNewsItem | null>(null);
  const currentLocalDayKey = localDayKeyFromDate(new Date());
  const previousLocalDayKey = localDayKeyDaysAgo(1);
  const {
    dailyFeedHistory,
    setDailyFeedHistory,
    dailyTestRunHistory,
    setDailyTestRunHistory,
    isDailyHistoryLoading,
    dailyHistoryError,
    setDailyHistoryError,
  } = useDailyNewsHistory({
    wsUrl,
    authSession,
    currentLocalDayKey,
    previousLocalDayKey,
    reloadVersion,
    onLoaded: ({ feedItems, testRunItems }) => {
      setLastItem((previousLastItem) =>
        pickNewestKnownItem([previousLastItem, feedItems[0], testRunItems[0]]),
      );
    },
  });

  useEffect(() => {
    historyRef.current = history;
  }, [history]);

  useEffect(() => {
    dailyFeedHistoryRef.current = dailyFeedHistory;
  }, [dailyFeedHistory]);

  useEffect(() => {
    dailyTestRunHistoryRef.current = dailyTestRunHistory;
  }, [dailyTestRunHistory]);

  useEffect(() => {
    lastItemRef.current = lastItem;
  }, [lastItem]);

  useEffect(() => {
    setIsPreviousFeedDayVisible(false);
  }, [currentLocalDayKey, wsUrl]);

  useEffect(() => {
    if (!authSession) {
      setNewsSourceStatuses([]);
      setCalendarDaySummaryStatus(null);
    }
  }, [authSession]);

  const recordTranslatedNewsItem = useCallback(
    (item: TranslatedNewsItem) => {
      if (realFeedSources.has(item.source) && !isDisplayableRealFeedItem(item)) {
        return;
      }

      setLastItem(item);
      setHistory((previousHistory) =>
        sortHistoryNewestFirst([item, ...previousHistory]).slice(
          0,
          MAX_HISTORY_ITEMS,
        ),
      );

      if (
        localDayKeyFromTimestamp(itemTimestampValue(item)) !==
        currentLocalDayKey
      ) {
        return;
      }

      if (item.test_run_id) {
        setDailyTestRunHistory((previousHistory) =>
          mergeHistoryItemsNewestFirst(previousHistory, [item]).slice(0, 200),
        );
        return;
      }

      if (realFeedSources.has(item.source)) {
        setDailyFeedHistory((previousHistory) =>
          dedupeRealFeedItemsBySourcePriority(
            mergeHistoryItemsNewestFirst(previousHistory, [item]),
          ).slice(0, 300),
        );
      }
    },
    [currentLocalDayKey, setDailyFeedHistory, setDailyTestRunHistory],
  );

  const resetNewsHistoryRuntime = useCallback(() => {
    setHistory([]);
    setDailyFeedHistory([]);
    setDailyTestRunHistory([]);
    setDailyHistoryError(null);
    setLastItem(null);
    setNewsSourceStatuses([]);
    setCalendarDaySummaryStatus(null);
  }, [setDailyFeedHistory, setDailyHistoryError, setDailyTestRunHistory]);

  const recordCalendarDaySummaryStatus = useCallback(
    (summaryStatus: PublicCalendarDaySummaryStatus) => {
      setCalendarDaySummaryStatus(summaryStatus);
    },
    [],
  );

  const recordNewsSourceStatuses = useCallback(
    (sourceStatuses: PublicSourceStatus[]) => {
      setNewsSourceStatuses(sourceStatuses);
    },
    [],
  );

  const latestKnownItem = pickNewestKnownItem([
    lastItem,
    dailyFeedHistory[0],
    dailyTestRunHistory[0],
    history[0],
  ]);
  const findDeliveredTestItem = useCallback(
    (runId: string | null | undefined) => {
      if (!runId) {
        return null;
      }

      return (
        dailyTestRunHistory.find((item) => item.test_run_id === runId) ||
        history.find((item) => item.test_run_id === runId) ||
        (lastItem?.test_run_id === runId ? lastItem : null)
      );
    },
    [dailyTestRunHistory, history, lastItem],
  );

  return {
    calendarDaySummaryStatus,
    currentLocalDayKey,
    dailyFeedHistory,
    dailyFeedHistoryRef,
    dailyHistoryError,
    dailyTestRunHistory,
    dailyTestRunHistoryRef,
    findDeliveredTestItem,
    history,
    historyRef,
    isDailyHistoryLoading,
    isPreviousFeedDayVisible,
    lastItem,
    lastItemRef,
    latestKnownItem,
    newsSourceStatuses,
    previousLocalDayKey,
    recordCalendarDaySummaryStatus,
    recordTranslatedNewsItem,
    recordNewsSourceStatuses,
    resetNewsHistoryRuntime,
    setIsPreviousFeedDayVisible,
  };
}
