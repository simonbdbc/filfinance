import type { AuthSessionClient } from "@shared/api/authSession";
import { useState } from "react";

import type {
  EdgeAudioFailedMessage,
  EdgeAudioFinalizedMessage,
  EdgeAudioReadyMessage,
  EdgeCalendarAlertReadyMessage,
  PublicCalendarDaySummaryStatus,
  PublicSourceStatus,
  TranslatedNewsItem,
} from "@shared/api/contracts";
import {
  buildEdgeLiveTicketUrl,
  defaultEdgeLiveWsUrl,
} from "@shared/api/urls";
import { useNewsSocket } from "./useNewsSocket";

type UseLiveMetadataRuntimeOptions = {
  authSession: AuthSessionClient | null;
  onAudioFailed: (message: EdgeAudioFailedMessage) => void;
  onAudioFinalized: (message: EdgeAudioFinalizedMessage) => void;
  onAudioReady: (message: EdgeAudioReadyMessage) => void;
  onCalendarAlertReady: (message: EdgeCalendarAlertReadyMessage) => void;
  onCalendarDaySummary: (summary: PublicCalendarDaySummaryStatus) => void;
  onSourceStatuses: (sources: PublicSourceStatus[]) => void;
  onTranslatedNewsItem: (item: TranslatedNewsItem) => void;
  reloadVersion: number;
  wsUrl: string;
};

export function useLiveMetadataRuntime({
  authSession,
  onAudioFailed,
  onAudioFinalized,
  onAudioReady,
  onCalendarAlertReady,
  onCalendarDaySummary,
  onSourceStatuses,
  onTranslatedNewsItem,
  reloadVersion,
  wsUrl,
}: UseLiveMetadataRuntimeOptions) {
  const [edgeLiveWsUrl] = useState(() => defaultEdgeLiveWsUrl());
  const liveMetadataWsUrl = edgeLiveWsUrl || wsUrl;
  const liveMetadataFallbackWsUrl = edgeLiveWsUrl ? wsUrl : null;
  const edgeLiveTicketUrl = edgeLiveWsUrl
    ? buildEdgeLiveTicketUrl(wsUrl)
    : null;
  const {
    connectionState,
    activeWsUrl,
    isUsingFallback,
    lastLaggedCount,
    lastError,
    resetNewsSocketState,
  } = useNewsSocket({
    wsUrl: liveMetadataWsUrl,
    fallbackWsUrl: liveMetadataFallbackWsUrl,
    edgeTicketUrl: edgeLiveTicketUrl,
    authSession,
    reloadVersion,
    onAudioReady,
    onCalendarAlertReady,
    onCalendarDaySummary,
    onAudioFailed,
    onAudioFinalized,
    onSourceStatuses,
    onTranslatedNewsItem,
  });

  return {
    activeLiveMetadataWsUrl: activeWsUrl,
    connectionState,
    isLiveMetadataEdgeEnabled: Boolean(edgeLiveWsUrl),
    isLiveMetadataFallbackActive: isUsingFallback,
    lastError,
    lastLaggedCount,
    resetLiveMetadataState: resetNewsSocketState,
  };
}
