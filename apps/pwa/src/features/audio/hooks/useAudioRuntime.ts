import type { AuthSessionClient } from "@shared/api/authSession";
import { useEffect, useRef, useState } from "react";
import type { MutableRefObject } from "react";

import type {
  PublicCalendarDaySummaryStatus,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import { useAudioPlayback } from "./useAudioPlayback";

type UseAudioRuntimeOptions = {
  authSession: AuthSessionClient | null;
  calendarDaySummaryStatus: PublicCalendarDaySummaryStatus | null;
  dailyFeedHistoryRef: MutableRefObject<TranslatedNewsItem[]>;
  dailyTestRunHistoryRef: MutableRefObject<TranslatedNewsItem[]>;
  historyRef: MutableRefObject<TranslatedNewsItem[]>;
  isWebPlatform: boolean;
  lastItemRef: MutableRefObject<TranslatedNewsItem | null>;
  latestKnownItem: TranslatedNewsItem | null;
  wsUrl: string;
};

export function useAudioRuntime({
  authSession,
  calendarDaySummaryStatus,
  dailyFeedHistoryRef,
  dailyTestRunHistoryRef,
  historyRef,
  isWebPlatform,
  lastItemRef,
  latestKnownItem,
  wsUrl,
}: UseAudioRuntimeOptions) {
  const [queue, setQueue] = useState<TranslatedNewsItem[]>([]);
  const [currentItem, setCurrentItem] = useState<TranslatedNewsItem | null>(
    null,
  );
  const queueRef = useRef<TranslatedNewsItem[]>([]);
  const playback = useAudioPlayback({
    isWebPlatform,
    wsUrl,
    authSession,
    calendarDaySummaryStatus,
    queueRef,
    historyRef,
    dailyFeedHistoryRef,
    dailyTestRunHistoryRef,
    lastItemRef,
    setQueue,
    setCurrentItem,
  });

  useEffect(() => {
    queueRef.current = queue;
  }, [queue]);

  const clearCurrentAndQueue = () => {
    playback.stopAudioPlayback();
    setCurrentItem(null);
    setQueue([]);
  };
  const clearCurrentItem = () => {
    playback.stopAudioPlayback();
    setCurrentItem(null);
  };

  return {
    ...playback,
    clearCurrentAndQueue,
    clearCurrentItem,
    currentItem,
    currentOrLatestItem: currentItem || latestKnownItem,
    isWebPlatform,
    queuedCount: queue.length,
  };
}
