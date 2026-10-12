export type KnownAppRole = "listener" | "moderator" | "admin";
export type AppRole = KnownAppRole | string;

export const appRoleDisplayLabelByRole: Record<KnownAppRole, string> = {
  listener: "Auditeur",
  moderator: "Modérateur",
  admin: "Admin",
};

export function appRoleDisplayLabel(role: AppRole | null | undefined) {
  if (!role) {
    return null;
  }

  return appRoleDisplayLabelByRole[role as KnownAppRole] ?? role;
}

export type AppView =
  | "feed"
  | "calendar"
  | "profile"
  | "admin"
  | "adminConfigSources"
  | "adminFluxStatus"
  | "adminBlacklist"
  | "adminGlossary"
  | "adminHistory"
  | "adminCalendarHistory"
  | "adminDailySummary"
  | "adminLogs";

export function canAccessAdmin(role: AppRole | null) {
  return role === "admin" || role === "moderator";
}

export function canAccessAppView(view: AppView, role: AppRole | null) {
  if (
    view === "admin" ||
    view === "adminConfigSources" ||
    view === "adminFluxStatus" ||
    view === "adminBlacklist" ||
    view === "adminGlossary" ||
    view === "adminHistory" ||
    view === "adminCalendarHistory" ||
    view === "adminDailySummary" ||
    view === "adminLogs"
  ) {
    return canAccessAdmin(role);
  }

  return true;
}
