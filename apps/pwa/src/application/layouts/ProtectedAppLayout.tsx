import { Redirect, Slot, usePathname, useRouter } from "expo-router";

import { useAppRuntime } from "../hooks/useAppRuntime";
import { routeByView, viewFromPathname } from "../navigation/routes";
import { AppRuntimeProvider } from "../providers/AppRuntimeProvider";
import { AppLayout } from "./AppLayout";
import { AuthTokenRecoveryScreen, useAuth } from "@features/auth";
import { ErrorSummaryCard } from "@features/system";
import { canAccessAppView } from "@shared/auth/roles";

function RoutedAppLayout() {
  const runtime = useAppRuntime();
  const pathname = usePathname();
  const router = useRouter();
  const activeView = viewFromPathname(pathname);
  const shouldShowErrorSummary = runtime.canAccessAdmin;

  return (
    <AppLayout
      activeView={activeView}
      pageKey={pathname}
      audioControl={runtime.audioControl}
      role={runtime.role}
      onThemeNameChange={runtime.theme.setThemeName}
      onViewChange={(view) => {
        if (!canAccessAppView(view, runtime.role)) {
          router.replace("/feed");
          return;
        }

        router.push(routeByView[view]);
      }}
    >
      {shouldShowErrorSummary ? (
        <ErrorSummaryCard
          fluxError={runtime.errors.fluxError}
          audioError={runtime.errors.audioError}
          historyError={runtime.errors.historyError}
          traceError={runtime.errors.traceError}
          profileError={runtime.errors.profileError}
        />
      ) : null}
      <Slot />
    </AppLayout>
  );
}

export function ProtectedAppLayout() {
  const { authStatus, authSession, user } = useAuth();

  if (authStatus === "loading") {
    return null;
  }

  if (authStatus !== "authenticated") {
    if (user) {
      return <AuthTokenRecoveryScreen />;
    }
    return <Redirect href="/login" />;
  }

  return (
    <AppRuntimeProvider key={authSession!.id}>
      <RoutedAppLayout />
    </AppRuntimeProvider>
  );
}
