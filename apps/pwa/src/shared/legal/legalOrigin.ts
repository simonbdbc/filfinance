// Aucun jeton, chemin ou identifiant n'est ajouté à l'origine des documents publics.
export function parseLegalOrigin(value: string | undefined) {
  if (!value?.trim()) throw new Error("EXPO_PUBLIC_LANDING_ORIGIN manquante");
  const url = new URL(value.trim());
  const loopback = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
  if (url.protocol !== "https:" && !(loopback && url.protocol === "http:")) {
    throw new Error("L’origine des documents doit utiliser HTTPS, sauf en local.");
  }
  if (url.username || url.password || url.pathname !== "/" || url.search || url.hash) {
    throw new Error("Une origine sans identifiants, chemin, paramètres ou fragment est attendue.");
  }
  return url.origin;
}

export function legalLinksForOrigin(value: string | undefined) {
  const origin = parseLegalOrigin(value);
  return [
    { key: "notice", label: "Mentions légales", href: `${origin}/mentions-legales/` },
    { key: "privacy", label: "Confidentialité", href: `${origin}/politique-de-confidentialite/` },
    { key: "terms", label: "CGU", href: `${origin}/conditions-generales-utilisation/` },
    { key: "cookies", label: "Cookies", href: `${origin}/cookies/` },
  ];
}
