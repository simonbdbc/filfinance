import type { AppView } from "@shared/auth/roles";

export const appViewDisplayNameByView: Record<AppView, string> = {
  admin: "Supervision admin",
  adminConfigSources: "Configuration de X",
  adminFluxStatus: "Flux en direct",
  adminBlacklist: "Liste de blocage",
  adminGlossary: "Dictionnaire et diction TTS",
  adminHistory: "Historique des actualités",
  adminCalendarHistory: "Historique du calendrier",
  adminDailySummary: "Traitement du calendrier",
  adminLogs: "Journaux",
  calendar: "Calendrier économique",
  feed: "Actualités",
  profile: "Mon profil",
};

export const routeByView: Record<AppView, string> = {
  admin: "/admin",
  adminConfigSources: "/admin/config-sources",
  adminFluxStatus: "/admin/flux-status",
  adminBlacklist: "/admin/blacklist",
  adminGlossary: "/admin/glossary",
  adminHistory: "/admin/history",
  adminCalendarHistory: "/admin/calendar-history",
  adminDailySummary: "/admin/calendar-day",
  adminLogs: "/admin/logs",
  calendar: "/calendar",
  feed: "/feed",
  profile: "/profile",
};

export function viewFromPathname(pathname: string): AppView {
  if (pathname.startsWith("/admin/config-sources")) {
    return "adminConfigSources";
  }

  if (pathname.startsWith("/admin/logs")) {
    return "adminLogs";
  }

  if (pathname.startsWith("/admin/glossary")) {
    return "adminGlossary";
  }

  if (pathname.startsWith("/admin/blacklist")) {
    return "adminBlacklist";
  }

  if (pathname.startsWith("/admin/flux-status")) {
    return "adminFluxStatus";
  }

  if (pathname.startsWith("/admin/history")) {
    return "adminHistory";
  }

  if (pathname.startsWith("/admin/calendar-history")) {
    return "adminCalendarHistory";
  }

  if (
    pathname.startsWith("/admin/calendar-day") ||
    pathname.startsWith("/admin/daily-summary")
  ) {
    return "adminDailySummary";
  }

  if (pathname.startsWith("/admin")) {
    return "admin";
  }
  if (pathname.startsWith("/calendar")) {
    return "calendar";
  }
  if (pathname.startsWith("/feed")) {
    return "feed";
  }
  if (pathname.startsWith("/profile")) {
    return "profile";
  }
  return "feed";
}
