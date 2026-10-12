import type { AuthSessionClient } from "@shared/api/authSession";
import { useState } from "react";

import type { TranslatedNewsItem } from "@shared/api/contracts";
import { useTestRunTraceSocket } from "./useTestRunTraceSocket";

type UseTestRunRuntimeOptions = {
  authSession: AuthSessionClient | null;
  findDeliveredTestItem: (
    runId: string | null | undefined,
  ) => TranslatedNewsItem | null;
  isEnabled: boolean;
  onBeforeTrigger: () => void;
  testRunUnavailableReason: string;
  traceWsUrl: string | null;
};

export function useTestRunRuntime({
  authSession,
  findDeliveredTestItem,
  isEnabled,
  onBeforeTrigger,
  testRunUnavailableReason,
  traceWsUrl,
}: UseTestRunRuntimeOptions) {
  const [isTracePanelCollapsed, setIsTracePanelCollapsed] = useState(true);
  const [isTestRunHistoryCollapsed, setIsTestRunHistoryCollapsed] =
    useState(true);
  const {
    traceConnectionState,
    isTriggeringTestRun,
    testTraceItems,
    activeTestRun,
    lastTraceLaggedCount,
    lastTraceError,
    triggerTestRun,
    resetTraceState,
  } = useTestRunTraceSocket({
    traceWsUrl,
    authSession,
    isEnabled,
    onBeforeTrigger,
  });

  return {
    activeTestRun,
    canUseTestRunSocket: isEnabled,
    deliveredTestItem: findDeliveredTestItem(activeTestRun?.run_id),
    isTestRunHistoryCollapsed,
    isTracePanelCollapsed,
    isTriggeringTestRun,
    lastTraceError,
    lastTraceItem: testTraceItems[testTraceItems.length - 1] || null,
    lastTraceLaggedCount,
    resetTraceState,
    setIsTestRunHistoryCollapsed,
    setIsTracePanelCollapsed,
    testRunUnavailableReason,
    testTraceItems,
    traceConnectionState,
    traceWsUrl,
    triggerTestRun,
  };
}
