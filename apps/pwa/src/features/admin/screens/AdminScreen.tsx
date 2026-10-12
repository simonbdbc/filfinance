import { useRouter } from "expo-router";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import { AccessDeniedCard, UserProfileLoadingCard } from "@shared/ui";
import { AdminPage } from "../components/AdminPage";
import { useAdminSourceStatuses } from "../hooks/useAdminSourceStatuses";
import {
  useFinancialJuiceBrowserRuntime,
  useFinancialJuiceRuntime,
} from "../hooks/useFinancialJuiceRuntime";
import { useXRuntime } from "../hooks/useXRuntime";

export function AdminScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const router = useRouter();
  const financialJuiceRuntime = useFinancialJuiceRuntime({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });
  const financialJuiceBrowserRuntime = useFinancialJuiceBrowserRuntime({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });
  const xRuntime = useXRuntime({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });
  const canLoadAdminSourceDetails = role === "admin";
  const sourceStatuses = useAdminSourceStatuses({
    authSession,
    isEnabled: canLoadAdminSourceDetails,
    wsUrl: server.wsUrl,
  });

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  return (
    <AdminPage
      title={appViewDisplayNameByView.admin}
      blacklistLabel={appViewDisplayNameByView.adminBlacklist}
      configSourcesLabel={appViewDisplayNameByView.adminConfigSources}
      fluxStatusLabel={appViewDisplayNameByView.adminFluxStatus}
      glossaryLabel={appViewDisplayNameByView.adminGlossary}
      historyDebugLabel={appViewDisplayNameByView.adminHistory}
      dailySummaryLabel={appViewDisplayNameByView.adminDailySummary}
      logsLabel={appViewDisplayNameByView.adminLogs}
      financialJuiceRuntime={financialJuiceRuntime}
      financialJuiceBrowserRuntime={financialJuiceBrowserRuntime}
      newsSourceStatuses={sourceStatuses.statuses}
      newsSourceStatusesError={
        canLoadAdminSourceDetails
          ? sourceStatuses.error
          : "Détails des sources réservés au rôle admin."
      }
      isNewsSourceStatusesLoading={sourceStatuses.isLoading}
      onReloadNewsSourceStatuses={sourceStatuses.reload}
      onOpenConfigSources={() => router.push("/admin/config-sources")}
      onOpenBlacklist={() => router.push("/admin/blacklist")}
      onOpenFluxStatus={() => router.push("/admin/flux-status")}
      onOpenGlossary={() => router.push("/admin/glossary")}
      onOpenHistoryDebug={() => router.push("/admin/history")}
      onOpenDailySummary={() => router.push("/admin/calendar-day")}
      onOpenLogs={() => router.push("/admin/logs")}
      role={role}
      xRuntime={xRuntime}
    />
  );
}
