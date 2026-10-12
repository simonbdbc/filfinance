import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { parseHTML } from "linkedom";
import sharp from "sharp";

import { siteConfig } from "../src/config/site.config.ts";
import { validateScripts } from "./validate-scripts.mjs";
import { legalConfig, legalPages } from "../src/config/legal.config.ts";
import { validateLegal } from "./validate-legal.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const dist = path.join(root, "dist");
const indexPath = path.join(dist, "index.html");
const errors = [];
// La seule exception applicative est comparée à sa source locale, pas à un simple marqueur.
const themeSource = await readFile(new URL("../src/scripts/theme.js", import.meta.url), "utf8");

const fail = (message) => errors.push(message);
const assert = (condition, message) => {
  if (!condition) fail(message);
};

const fileExists = async (filePath) => {
  try {
    return (await stat(filePath)).isFile();
  } catch {
    return false;
  }
};

assert(await fileExists(indexPath), "dist/index.html est absent.");

if (errors.length === 0) {
  const html = await readFile(indexPath, "utf8");
  const { document } = parseHTML(html);
  const meta = (selector) => document.querySelector(selector)?.getAttribute("content")?.trim() ?? "";

  assert(document.title.trim() === siteConfig.seo.title, "Le title ne correspond pas à la configuration.");
  assert(meta('meta[name="description"]') === siteConfig.seo.description, "La meta description est absente ou incorrecte.");
  assert(meta('meta[name="robots"]') === "index, follow", "La directive robots doit être index, follow.");

  const canonical = document.querySelector('link[rel="canonical"]')?.getAttribute("href") ?? "";
  assert(canonical === siteConfig.origin, "L’URL canonical est absente ou incorrecte.");

  const socialImage = new URL(siteConfig.images.social, siteConfig.origin).toString();
  const requiredMeta = new Map([
    ['meta[property="og:type"]', "website"],
    ['meta[property="og:title"]', siteConfig.seo.title],
    ['meta[property="og:description"]', siteConfig.seo.description],
    ['meta[property="og:url"]', siteConfig.origin],
    ['meta[property="og:image"]', socialImage],
    ['meta[property="og:image:width"]', "1200"],
    ['meta[property="og:image:height"]', "630"],
    ['meta[name="twitter:card"]', "summary_large_image"],
    ['meta[name="twitter:title"]', siteConfig.seo.title],
    ['meta[name="twitter:description"]', siteConfig.seo.description],
    ['meta[name="twitter:image"]', socialImage],
  ]);

  for (const [selector, expected] of requiredMeta) {
    assert(meta(selector) === expected, `${selector} est absent ou incorrect.`);
  }

  assert(document.querySelectorAll("h1").length === 1, "Le document doit contenir exactement un H1.");

  const jsonLdScripts = [...document.querySelectorAll('script[type="application/ld+json"]')];
  assert(jsonLdScripts.length >= 1, "Les données structurées JSON-LD sont absentes.");
  for (const script of jsonLdScripts) {
    try {
      JSON.parse(script.textContent ?? "");
    } catch {
      fail("Un bloc JSON-LD est invalide.");
    }
  }

  validateScripts(document, themeSource).forEach(fail);

  // Le build vérifie aussi le contrat de la nouvelle landing, sans dépendre d'une API.
  const sectionIds = [...document.querySelectorAll("main > section")].map((section) => section.id);
  assert(JSON.stringify(sectionIds) === JSON.stringify(["", "offre", "flux-audio", "actualites", "resume-audio", "calendrier"]), "L’ordre des sections de la landing est incorrect.");
  assert(document.querySelectorAll(".service").length === 4, "Les quatre services doivent être présentés.");
  assert(document.querySelectorAll("#calendrier thead th").length === 7, "Le calendrier doit conserver sept jours.");
  assert(document.querySelectorAll(".brand .brand-icon").length === 2, "L’icône audio doit figurer dans les deux zones de marque.");
  assert(document.querySelectorAll("audio, video, main img").length === 0, "Les aperçus ne doivent pas embarquer de média réel ni de photographie.");
  assert(document.documentElement.getAttribute("data-theme") === "light", "Le HTML initial doit être clair.");
  assert(meta('meta[name="theme-color"]') === siteConfig.themeColors.light, "La couleur initiale du navigateur doit être claire.");
  const toggle = document.getElementById("theme-toggle");
  assert(toggle?.hasAttribute("hidden") && toggle?.getAttribute("aria-pressed") === "false", "La commande doit attendre son script avant d’être activée.");

  for (const anchor of document.querySelectorAll('a[href^="#"]')) {
    const href = anchor.getAttribute("href") ?? "";
    const id = href.slice(1);
    assert(Boolean(id) && document.getElementById(id) !== null, `L’ancre ${href} ne cible aucun élément.`);
  }

  const expectedCtas = {
    home: siteConfig.externalDestinations.home,
    login: siteConfig.externalDestinations.login,
  };
  for (const anchor of document.querySelectorAll("a[data-cta]")) {
    const kind = anchor.getAttribute("data-cta");
    const href = anchor.getAttribute("href") ?? "";
    const expected = expectedCtas[kind];
    assert(Boolean(expected) && href === expected, `Le CTA ${kind ?? "inconnu"} ne correspond pas à la configuration.`);
  }

  const internalAssets = new Set();
  for (const element of document.querySelectorAll("img[src], link[href], script[src]")) {
    const value = element.getAttribute("src") ?? element.getAttribute("href") ?? "";
    if (value.startsWith("/") && !value.startsWith("//")) internalAssets.add(value);
  }
  for (const source of document.querySelectorAll("source[srcset]")) {
    const values = (source.getAttribute("srcset") ?? "")
      .split(",")
      .map((candidate) => candidate.trim().split(/\s+/u)[0])
      .filter((value) => value.startsWith("/") && !value.startsWith("//"));
    values.forEach((value) => internalAssets.add(value));
  }

  for (const asset of internalAssets) {
    const assetPath = path.join(dist, decodeURIComponent(asset).replace(/^\/+/, ""));
    assert(await fileExists(assetPath), `L’asset référencé ${asset} est absent du build.`);
  }

  const externalBlockingResources = [
    ...document.querySelectorAll('link[rel="stylesheet"][href^="http"]'),
    ...document.querySelectorAll('img[src^="http"]'),
    ...document.querySelectorAll('script[src^="http"]'),
  ];
  assert(externalBlockingResources.length === 0, "Une ressource tierce bloquante est présente.");
}

const robotsPath = path.join(dist, "robots.txt");
assert(await fileExists(robotsPath), "robots.txt est absent.");
if (await fileExists(robotsPath)) {
  const robots = await readFile(robotsPath, "utf8");
  assert(robots.includes(`Sitemap: ${siteConfig.origin}sitemap-index.xml`), "robots.txt ne référence pas le sitemap.");
}

assert(await fileExists(path.join(dist, "sitemap-index.xml")), "sitemap-index.xml est absent.");

const socialPath = path.join(dist, siteConfig.images.social.replace(/^\//u, ""));
assert(await fileExists(socialPath), "L’image sociale est absente.");
if (await fileExists(socialPath)) {
  const metadata = await sharp(socialPath).metadata();
  assert(metadata.width === 1200 && metadata.height === 630, "L’image sociale doit mesurer 1200 × 630 px.");
}

const imageExtensions = new Set([".avif", ".webp", ".jpg", ".jpeg", ".png"]);
const imageBudgetBytes = 600_000;

const walk = async (directory) => {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(entryPath)));
    if (entry.isFile()) files.push(entryPath);
  }
  return files;
};

for (const file of await walk(dist)) {
  if (!imageExtensions.has(path.extname(file).toLowerCase())) continue;
  const size = (await stat(file)).size;
  assert(size <= imageBudgetBytes, `${path.relative(dist, file)} dépasse le budget image de ${imageBudgetBytes} octets.`);
}

// Lire les sorties statiques des quatre routes, sans fallback vers l'accueil.
const legalDocuments = {};
for (const page of legalPages) {
  const file = path.join(dist, page.path, "index.html");
  if (await fileExists(file)) legalDocuments[page.path] = await readFile(file, "utf8");
}
const sitemapFiles = (await readdir(dist)).filter(name => /^sitemap-\d+\.xml$/u.test(name));
const sitemap = (await Promise.all(sitemapFiles.map(name => readFile(path.join(dist, name), "utf8")))).join("\n");
validateLegal({ env: process.env, pages: legalPages, publisher: legalConfig.publisher, site: siteConfig, documents: legalDocuments, homeHtml: await readFile(indexPath, "utf8"), sitemap, themeSource }).forEach(fail);

if (errors.length > 0) {
  console.error(errors.map((error) => `- ${error}`).join("\n"));
  process.exit(1);
}

console.log("[validate] Build statique, pages légales, SEO, CTA, ancres, scripts et budgets : OK.");
