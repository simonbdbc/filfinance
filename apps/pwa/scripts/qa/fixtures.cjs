const day = new Date().toLocaleDateString("en-CA", {
  timeZone: "Europe/Paris",
});
const user = {
  id: "ui-qa-user",
  name: "Recette locale",
  email: "recette@example.test",
  emailVerified: true,
};
const titles = [
  "Les marchés européens dans l’attente des prochains indicateurs.",
  "Banques centrales : les prises de parole à suivre.",
  "Devises : les annonces économiques au premier plan.",
  "Les valeurs technologiques sous surveillance.",
  "Économie : les rendez-vous de la semaine.",
  "Matières premières : le point du matin.",
];
function newsForDay(date) {
  return titles.map((title, i) => ({
    id: i + 1 + (date === day ? 0 : 100),
    news_event_id: i + 1 + (date === day ? 0 : 100),
    language: "fr",
    status: "translated",
    source: "financial_juice",
    event_kind: "news",
    source_title: title,
    source_body: "Source illustrative.",
    translated_title: title,
    translated_body:
      "Les investisseurs suivent les prochains rendez-vous économiques. Cette actualité sert uniquement à illustrer la présentation du fil pendant la recette locale.",
    narration_text: title,
    translation_mode: "auto",
    error_message: null,
    news_level: "news-general",
    created_at: `${date}T09:${42 - i * 5}:00+02:00`,
    updated_at: `${date}T09:${42 - i * 5}:00+02:00`,
    url: "https://example.test/source",
  }));
}
function fixture(url, state = {}) {
  const p = url.pathname,
    role = state.role ?? "listener",
    authenticated = state.authenticated !== false;
  if (state.detailError && /^\/api\/translations\/\d+$/.test(p))
    return { status: 503, body: { message: "Erreur de détail simulée" } };
  if (p.endsWith("/get-session"))
    return {
      body: authenticated
        ? {
            user,
            session: {
              id: "local-session",
              userId: user.id,
              expiresAt: new Date(Date.now() + 3600000).toISOString(),
              token: "local-fixture",
            },
          }
        : null,
    };
  if (p.endsWith("/token"))
    return {
      body: {
        token: `eyJhbGciOiJSUzI1NiJ9.${Buffer.from(JSON.stringify({ sub: user.id, exp: Math.floor(Date.now() / 1000) + 3600, email_verified: true })).toString("base64url")}.local-test-signature`,
      },
    };
  if (p === "/api/me")
    return {
      body: {
        id: user.id,
        email: user.email,
        role,
        preferences: { theme: "dark", preferred_tts_voice: "Français" },
        created_at: "2026-09-01T00:00:00Z",
        updated_at: "2026-09-15T00:00:00Z",
      },
    };
  if (p === "/api/runtime/financial-juice")
    return {
      body: {
        source: "financial_juice",
        enabled: true,
        configured_enabled: true,
        disabled_reason: null,
      },
    };
  if (p === "/api/runtime/financial-juice-browser")
    return {
      body: {
        source: "financial_juice_browser",
        configured_enabled: true,
        primary_enabled: false,
        fallback_active: true,
        ingest_token_configured: true,
        status: "ok",
        message: null,
        updated_at: new Date().toISOString(),
        effective_enabled: true,
        service_accessible: true,
        main_feed_available: true,
        stale: false,
        effective_message: null,
      },
    };
  if (p === "/api/runtime/x")
    return {
      body: {
        source: "x",
        configured_enabled: false,
        enabled: false,
        disabled_reason: null,
        bearer_token_configured: true,
        persistence_available: true,
        account_usernames: ["exemple"],
        keywords: ["inflation"],
        updated_at: new Date().toISOString(),
      },
    };
  if (
    [
      "/api/glossary",
      "/api/news-blacklist",
      "/api/news-blacklist/hits",
      "/api/news-truncation-audit",
    ].includes(p)
  )
    return { body: { items: [] } };
  if (p === "/api/hls-ticket")
    return { body: { enabled: false, ticket: null } };
  if (p === "/api/replay/today")
    return {
      body: {
        enabled: true,
        available: false,
        local_day: day,
        playlist_url: "",
        segment_count: 0,
        first_sequence: null,
        latest_sequence: null,
        updated_at: new Date().toISOString(),
      },
    };
  if (p === "/api/translations")
    return {
      body: {
        items:
          state.empty ||
          url.searchParams.get("history_scope") === "test_run" ||
          url.searchParams.get("source") === "financial_juice_browser"
            ? []
            : newsForDay(url.searchParams.get("day") || day),
        has_more: false,
        next_before_id: null,
      },
    };
  if (/^\/api\/translations\/\d+$/.test(p))
    return {
      body: newsForDay(day).find((n) => n.id === Number(p.split("/").at(-1))),
    };
  if (p === "/api/sources")
    return {
      body: [
        {
          name: "financial_juice",
          source: "financial_juice",
          enabled: true,
          status: "active",
          updated_at: new Date().toISOString(),
          url: "https://example.test",
          poll_interval_ms: 5000,
          request_timeout_ms: 10000,
        },
        {
          name: "x",
          source: "x",
          enabled: false,
          status: "disabled",
          updated_at: new Date().toISOString(),
          url: "https://example.test",
          poll_interval_ms: 5000,
          request_timeout_ms: 10000,
        },
      ],
    };
  if (p === "/api/calendar-day" || p === "/api/calendar-day/today") {
    const date = url.searchParams.get("local_day") || day;
    const items = state.empty
      ? []
      : [
          "Indice des prix à la consommation",
          "Demandes d’allocations chômage",
          "Discours de la banque centrale",
        ].map((title, i) => ({
          id: i + 1,
          localDay: date,
          eventTime: `${date}T${14 + i}:30:00+02:00`,
          sourceTitle: title,
          translatedTitle: title,
          ttsText: title,
          country: "États-Unis",
          currency: "USD",
          forecastValue: "2,5 %",
          previousValue: "2,4 %",
          actualValue: null,
          sourceImportance: "high",
          aiImportance: "high",
          finalImportance: i === 2 ? "medium" : "high",
          category: "Inflation",
          reasoning: "Un indicateur à suivre pour la session.",
          confidence: 0.9,
          contentHash: "qa",
          sourcesJson: "[]",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        }));
    return {
      body: {
        localDay: date,
        state: { localDay: date, status: "ready" },
        itemCount: items.length,
        highItemCount: 2,
        items,
        summary: {
          title: "Votre journée économique",
          summaryText: "Inflation et banques centrales au programme.",
        },
        alerts: [],
        itemAudios: [],
      },
    };
  }
  if (p.endsWith("/summary"))
    return {
      body: {
        localDay: day,
        status: "missing",
        summaryId: null,
        title: null,
        summaryText: null,
        highEventCount: 0,
      },
    };
  return {
    status: 503,
    body: { message: "Service simulé indisponible pour la recette locale." },
  };
}
module.exports = { fixture, day };
