import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";
import { legalConfig, legalFields, legalPages } from "../src/config/legal.config.ts";
import { siteConfig } from "../src/config/site.config.ts";
import { validateLegal } from "./validate-legal.mjs";

// Les cas négatifs altèrent une copie en mémoire du véritable build, jamais les fichiers servis.
const dist = new URL("../dist/", import.meta.url);
const documents = {};
for (const page of legalPages) documents[page.path] = await readFile(new URL(`.${page.path}index.html`, dist), "utf8");
const input = {
  env: { ...legalFields, ...Object.fromEntries(Object.keys(legalFields).filter(key => process.env[key] !== undefined).map(key => [key, process.env[key]])), PUBLIC_LEGAL_PUBLICATION_ENFORCED: "true" },
  publisher: legalConfig.publisher, pages: legalPages, site: siteConfig, documents,
  homeHtml: await readFile(new URL("index.html", dist), "utf8"),
  sitemap: (await Promise.all((await readdir(dist)).filter(name => /^sitemap-\d+\.xml$/u.test(name)).map(name => readFile(new URL(name, dist), "utf8")))).join("\n"),
  themeSource: await readFile(new URL("../src/scripts/theme.js", import.meta.url), "utf8"),
};
test("les quatre pages construites satisfont le contrôle strict", () => assert.deepEqual(validateLegal(input), []));
for (const key of Object.keys(legalFields)) {
  test(`publication refusée sans ${key}`, () => {
    const copy = structuredClone(input); delete copy.env[key];
    assert(validateLegal(copy).some(error => error.includes(key)));
  });
}
const route = legalPages[0].path;
for (const [name, mutate, message] of [
  ["variable provisoire", data => { data.env.PUBLIC_LEGAL_ENTITY_NAME = "{{EDITEUR}}"; }, "Publication"],
  ["page absente", data => { delete data.documents[route]; }, "Page légale absente"],
  ["fallback vers l’accueil", data => { data.documents[route] = data.homeHtml; }, "titre incorrect"],
  ["canonical incorrect", data => { data.documents[route] = data.documents[route].replace(`href="${siteConfig.origin}mentions-legales/"`, 'href="https://incorrect.invalid/"'); }, "canonical incorrect"],
  ["liens légaux retirés", data => { data.homeHtml = data.homeHtml.replaceAll('href="/cookies/"', 'href="/incorrect/"'); }, "lien légal /cookies/"],
  ["sitemap incomplet", data => { data.sitemap = ""; }, "absente du sitemap"],
  ["contenu provisoire", data => { data.documents[route] = data.documents[route].replace("</article>", "<p>__A_COMPLETER__</p></article>"); }, "contenu provisoire"],
  ["script ajouté", data => { data.documents[route] += "<script>alert(1)</script>"; }, "script"],
  ["information modifiée après build", data => { data.publisher.name = "Autre éditeur"; }, "ne correspond pas"],
]) test(`publication refusée : ${name}`, () => {
  const copy = structuredClone(input); mutate(copy);
  assert(validateLegal(copy).some(error => error.includes(message)));
});
