// Informations publiques retenues pour Fil Finance ; mêmes cinq champs que le socle ACREA.
export const legalFields = {
  PUBLIC_LEGAL_ENTITY_NAME: "SB ALGO",
  PUBLIC_LEGAL_REGISTERED_OFFICE: "60 rue François 1er, 75008 Paris",
  PUBLIC_LEGAL_PUBLICATION_DIRECTOR: "sbdev",
  PUBLIC_LEGAL_PRIVACY_CONTACT: "support@filfinance.fr",
  PUBLIC_LEGAL_DATA_RETENTION: "Les données du compte et des préférences sont conservées tant que le compte existe. Pour demander leur suppression, contactez support@filfinance.fr. La demande est examinée en tenant compte des obligations légales et des données nécessaires à la défense de droits ; aucun délai automatique de suppression n’est annoncé.",
} as const;

// Build Astro et validateur Node lisent le même environnement exporté.
const value = (key: keyof typeof legalFields) => process.env[key]?.trim() ?? legalFields[key];
export const legalConfig = {
  updatedAt: "18 septembre 2026",
  updatedAtIso: "2026-09-18",
  publisher: {
    name: value("PUBLIC_LEGAL_ENTITY_NAME"),
    registeredOffice: value("PUBLIC_LEGAL_REGISTERED_OFFICE"),
    publicationDirector: value("PUBLIC_LEGAL_PUBLICATION_DIRECTOR"),
    privacyContact: value("PUBLIC_LEGAL_PRIVACY_CONTACT"),
    dataRetention: value("PUBLIC_LEGAL_DATA_RETENTION"),
  },
  hosting: {
    name: "BunnyWay d.o.o.",
    address: "Dunajska cesta 165, 1000 Ljubljana, Slovénie",
    website: "https://bunny.net/",
    contact: "support@bunny.net",
  },
} as const;

export const legalPages = [
  { path: "/mentions-legales/", label: "Mentions légales", title: "Mentions légales", description: "Éditeur, publication, hébergement et contact de Fil Finance." },
  { path: "/politique-de-confidentialite/", label: "Confidentialité", title: "Politique de confidentialité", description: "Données personnelles, compte, préférences et droits des utilisateurs de Fil Finance." },
  { path: "/conditions-generales-utilisation/", label: "CGU", title: "Conditions générales d’utilisation", description: "Conditions d’accès et d’utilisation du site et de l’application Fil Finance." },
  { path: "/cookies/", label: "Cookies", title: "Cookies et stockages techniques", description: "Sessions, préférences de thème et filtres conservés par Fil Finance sur votre appareil." },
] as const;
