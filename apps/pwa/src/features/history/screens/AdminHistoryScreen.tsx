import { useCallback, useEffect, useState } from "react";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type { NewsTruncationAudit } from "@shared/api/contracts";
import { AccessDeniedCard, UserProfileLoadingCard } from "@shared/ui";
import { AdminFeedDiagnosticsCard } from "../components/AdminFeedDiagnosticsCard";
import { loadNewsTruncationAudits } from "../services/loadNewsTruncationAudits";

export function AdminHistoryScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, history, profile, role, server } = useAppRuntime();
  const [truncationAudits, setTruncationAudits] = useState<NewsTruncationAudit[]>([]);
  const [truncationAuditError, setTruncationAuditError] = useState<string | null>(null);
  const [isTruncationAuditLoading, setIsTruncationAuditLoading] = useState(false);

  const reloadTruncationAudits = useCallback(async () => {
    if (!canAccessAdmin) {
      return;
    }

    setIsTruncationAuditLoading(true);
    setTruncationAuditError(null);
    try {
      const response = await loadNewsTruncationAudits({
        authSession,
        wsUrl: server.wsUrl,
      });
      setTruncationAudits(response.items);
    } catch (nextError) {
      setTruncationAuditError(
        nextError instanceof Error
          ? nextError.message
          : "Impossible de charger les audits de troncature.",
      );
    } finally {
      setIsTruncationAuditLoading(false);
    }
  }, [authSession, canAccessAdmin, server.wsUrl]);

  useEffect(() => {
    void reloadTruncationAudits();
  }, [reloadTruncationAudits]);

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  return (
    <AdminFeedDiagnosticsCard
      title={appViewDisplayNameByView.adminHistory}
      items={history.dailyFeedHistory}
      isLoading={history.isDailyHistoryLoading}
      currentLocalDayKey={history.currentLocalDayKey}
      previousLocalDayKey={history.previousLocalDayKey}
      isPreviousDayVisible={history.isPreviousFeedDayVisible}
      onPreviousDayVisibilityChange={history.setIsPreviousFeedDayVisible}
      truncationAudits={truncationAudits}
      truncationAuditError={truncationAuditError}
      isTruncationAuditLoading={isTruncationAuditLoading}
      onReloadTruncationAudits={reloadTruncationAudits}
    />
  );
}
