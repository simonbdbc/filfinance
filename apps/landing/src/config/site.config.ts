// Les contenus et destinations publics restent indépendants des services de la PWA.
export type NavigationItem = { label: string; target: `#${string}` };
export type ServiceIcon = "audio" | "news" | "headphones" | "calendar";

const requireHttpsOrigin = (value: string, name: string) => {
  const url = new URL(value);
  if (url.protocol !== "https:") throw new Error(`${name} doit utiliser HTTPS.`);
  if (url.username || url.password || url.pathname !== "/" || url.search || url.hash) {
    throw new Error(`${name} doit être une origine sans identifiants, chemin, paramètres ni fragment.`);
  }
  return url.origin;
};

const publicOrigin = requireHttpsOrigin(process.env.FILFINANCE_LANDING_URL ?? "https://filfinance.fr", "FILFINANCE_LANDING_URL");
const pwaOrigin = requireHttpsOrigin(process.env.FILFINANCE_PWA_URL ?? "https://app.filfinance.fr", "FILFINANCE_PWA_URL");
export const externalDestinations = { home: `${pwaOrigin}/`, login: `${pwaOrigin}/login`, signup: null } as const;

export const siteConfig = {
  name: "Fil Finance",
  wordmark: "fil finance",
  locale: "fr-FR",
  lang: "fr",
  origin: `${publicOrigin}/`,
  seo: {
    title: "Fil Finance — L’actualité financière, en audio et en français",
    description: "Un flux audio d’actualités en français, un fil d’actualités financières, le résumé audio du jour et un calendrier économique hebdomadaire pour accompagner votre trading.",
  },
  // La métadonnée du navigateur suit les fonds définis dans les tokens CSS.
  themeColors: { light: "#F1E5D3", dark: "#111827" },
  navigation: [
    { label: "Flux audio", target: "#flux-audio" },
    { label: "Actualités", target: "#actualites" },
    { label: "Résumé audio", target: "#resume-audio" },
    { label: "Calendrier", target: "#calendrier" },
  ] satisfies NavigationItem[],
  hero: {
    eyebrow: "L’ACTUALITÉ FINANCIÈRE, EN AUDIO",
    heading: "Écoutez les marchés.",
    continuation: "Restez dans votre session.",
    description: [
      "Un flux d’actualités financières en français pour accompagner votre trading.",
      "Un calendrier hebdomadaire pour préparer vos rendez-vous.",
    ],
  },
  ctas: { primary: "Écouter le flux", secondary: "Se connecter" },
  offerHeading: "Ce que propose Fil Finance",
  services: [
    { icon: "audio", title: "Flux audio d’actualités en français", description: "Suivez les marchés à l’écoute." },
    { icon: "news", title: "Fil d’actualités financières", description: "Retrouvez les titres et leurs détails." },
    { icon: "headphones", title: "Résumé audio du jour en français", description: "Réécoutez l’essentiel de la journée." },
    { icon: "calendar", title: "Calendrier économique hebdomadaire", description: "Préparez les rendez-vous de la semaine." },
  ] satisfies { icon: ServiceIcon; title: string; description: string }[],
  audio: { title: "Flux audio d’actualités", example: "Français · Aperçu illustratif", caption: "Votre fil pendant la session" },
  news: {
    heading: "L’actualité, au fil des marchés.",
    description: "Consultez les actualités financières en français et ouvrez les détails qui vous intéressent.",
    cta: "Consulter le fil",
    title: "Fil d’actualités",
    example: "Aperçu illustratif",
    // Exemples fixes : aucune annonce économique réelle ni connexion à une API.
    items: [
      { time: "14:30", category: "Économie", title: "Indicateurs économiques" },
      { time: "14:00", category: "Banques centrales", title: "Décisions et prises de parole" },
      { time: "13:45", category: "Marchés", title: "Actualités des marchés" },
    ],
  },
  summary: {
    heading: "L’essentiel du jour, en audio.",
    description: "Retrouvez les principales actualités de la journée dans un résumé audio en français.",
    title: "Résumé audio du jour",
    example: "Français · Aperçu illustratif",
    cta: "Écouter le résumé",
  },
  calendar: {
    heading: "La semaine se prépare.",
    description: "Les rendez-vous économiques, réunis dans une vue hebdomadaire.",
    title: "Calendrier hebdomadaire",
    example: "Exemple de semaine",
    range: "7 – 13 septembre 2026",
    month: "septembre",
    // La date de cet exemple ne se met pas à jour indépendamment de son contenu.
    days: ["Lun. 7", "Mar. 8", "Mer. 9", "Jeu. 10", "Ven. 11", "Sam. 12", "Dim. 13"],
    rows: [
      ["Conjoncture", "Inflation", "Banques centrales", "Inflation", "Banques centrales", "—", "—"],
      ["Banques centrales", "Conjoncture", "—", "Conjoncture", "—", "—", "—"],
      ["—", "—", "—", "—", "—", "—", "—"],
    ],
  },
  images: {
    social: "/social/fil-finance-social.jpg",
    socialAlt: "Fil Finance : flux audio, actualités financières, résumé audio du jour et calendrier hebdomadaire.",
  },
  icons: { favicon32: "/icons/favicon-32.png", favicon64: "/icons/favicon-64.png", app192: "/icons/icon-192.png", appleTouch: "/icons/apple-touch-icon.png" },
  copyright: "© 2026 Fil Finance. Tous droits réservés.",
  externalDestinations,
} as const;
