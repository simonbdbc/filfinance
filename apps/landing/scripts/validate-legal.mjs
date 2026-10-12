import { parseHTML } from "linkedom";
import { legalFields } from "../src/config/legal.config.ts";
import { validateScripts } from "./validate-scripts.mjs";

// Valider le HTML réellement livré, pas seulement la présence de variables au build.
export function validateLegal({ env, pages, publisher, site, documents, homeHtml, sitemap, themeSource }) {
  const errors = [];
  const check = (condition, message) => { if (!condition) errors.push(message); };
  const provisional = /__A_COMPLETER__|\{\{|\bTODO\b|à (?:renseigner|compléter|confirmer)|acrea-fitness|acra-fitness/iu;
  const strict = env.PUBLIC_LEGAL_PUBLICATION_ENFORCED === "true";
  if (strict) {
    for (const key of Object.keys(legalFields)) {
      const value = env[key]?.trim();
      check(Boolean(value) && !provisional.test(value ?? ""), `Publication : ${key} manque ou contient une valeur provisoire.`);
    }
    check(/^[^\s@]+@[^\s@]+\.[^\s@]+$/u.test(publisher.privacyContact), "Le contact légal doit être une adresse email.");
  }
  const home = parseHTML(homeHtml).document;
  const expectedFields = Object.fromEntries(Object.keys(legalFields).map((key, index) => [key, [publisher.name, publisher.registeredOffice, publisher.publicationDirector, publisher.privacyContact, publisher.dataRetention][index]]));
  const foundFields = new Set();
  for (const page of [{ path: "/", title: null }, ...pages]) {
    const html = page.path === "/" ? homeHtml : documents[page.path];
    if (!html) { errors.push(`Page légale absente : ${page.path}`); continue; }
    const { document } = parseHTML(html);
    const footer = document.querySelector('footer nav[aria-label="Informations légales"]');
    check(Boolean(footer), `${page.path} : navigation légale absente.`);
    for (const target of pages) check(footer?.querySelectorAll(`a[href="${target.path}"]`).length === 1, `${page.path} : lien légal ${target.path} absent ou dupliqué.`);
    if (page.path === "/") continue;
    check(document.title === `${page.title} | ${site.name}`, `${page.path} : titre incorrect.`);
    check(document.documentElement.lang === "fr", `${page.path} : langue incorrecte.`);
    check(document.querySelectorAll("h1").length === 1 && document.querySelector("h1")?.textContent.trim() === page.title, `${page.path} : H1 incorrect.`);
    check(document.querySelector('meta[name="description"]')?.content === page.description, `${page.path} : description incorrecte.`);
    check(document.querySelector('meta[name="robots"]')?.content === "index, follow", `${page.path} : robots incorrect.`);
    const canonical = new URL(page.path, site.origin).href;
    check(document.querySelector('link[rel="canonical"]')?.getAttribute("href") === canonical, `${page.path} : canonical incorrect.`);
    check(document.querySelector('meta[property="og:url"]')?.content === canonical, `${page.path} : URL sociale incorrecte.`);
    check(sitemap.includes(`<loc>${canonical}</loc>`), `${page.path} : route absente du sitemap.`);
    check(Boolean(document.querySelector('main a[href="/"]')), `${page.path} : retour accueil absent.`);
    for (const item of site.navigation) check(Boolean(document.querySelector(`footer a[href="/${item.target}"]`)), `${page.path} : ancre d’accueil ${item.target} absente.`);
    for (const link of document.querySelectorAll('a[href^="#"], a[href^="/#"]')) {
      const href = link.getAttribute("href");
      const targetDocument = href.startsWith("/#") ? home : document;
      check(Boolean(targetDocument.getElementById(href.slice(href.indexOf("#") + 1))), `${page.path} : ancre invalide ${href}.`);
    }
    errors.push(...validateScripts(document, themeSource).map(error => `${page.path} : ${error}`));
    for (const script of document.querySelectorAll('script[type="application/ld+json"]')) {
      try { JSON.parse(script.textContent); } catch { errors.push(`${page.path} : JSON-LD invalide.`); }
    }
    check(document.querySelectorAll('script[src], link[rel="stylesheet"][href^="http"], link[rel="stylesheet"][href^="//"], iframe').length === 0, `${page.path} : ressource tierce inattendue.`);
    const toggle = document.getElementById("theme-toggle");
    check(document.documentElement.dataset.theme === "light" && toggle?.hasAttribute("hidden") && toggle?.getAttribute("aria-pressed") === "false", `${page.path} : thème initial incorrect.`);
    check(Boolean(document.querySelector("article time[datetime]")), `${page.path} : date éditoriale absente.`);
    for (const element of document.querySelectorAll("[data-legal-field]")) {
      const field = element.getAttribute("data-legal-field");
      foundFields.add(field);
      check(element.textContent.trim() === expectedFields[field], `${page.path} : ${field} ne correspond pas à la configuration.`);
    }
    if (strict) check(!provisional.test(document.body.textContent), `${page.path} : contenu provisoire ou ACREA résiduel.`);
  }
  for (const field of Object.keys(legalFields)) check(foundFields.has(field), `Information légale absente du HTML : ${field}.`);
  return errors;
}
