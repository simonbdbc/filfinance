import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import assert from "node:assert/strict";
import test from "node:test";
import { parseHTML } from "linkedom";
import { validateScripts } from "./validate-scripts.mjs";

const source = await readFile(new URL("../src/scripts/theme.js", import.meta.url), "utf8");
// Le DOM est encore en chargement : on distingue le thème précoce du raccordement du bouton.
const boot = ({ saved = null, failRead = false, failWrite = false } = {}) => {
  const { document, Event } = parseHTML('<!doctype html><html data-theme="light"><head><meta name="theme-color" content="#F1E5D3" data-light="#F1E5D3" data-dark="#111827"></head><body><button id="theme-toggle" aria-pressed="false" hidden></button></body></html>');
  Object.defineProperty(document, "readyState", { value: "loading" });
  const writes = [];
  const storage = {
    getItem(key) { assert.equal(key, "filfinance.landing.theme"); if (failRead) throw new Error("Stockage refusé"); return saved; },
    setItem(key, value) { if (failWrite) throw new Error("Stockage refusé"); writes.push([key, value]); },
  };
  runInNewContext(source, { document, window: { localStorage: storage } });
  const button = document.getElementById("theme-toggle");
  const mounted = () => document.dispatchEvent(new Event("DOMContentLoaded"));
  return { document, button, writes, mounted, theme: () => document.documentElement.dataset.theme, meta: () => document.querySelector("meta").content };
};

for (const [label, options, expected] of [
  ["première visite", {}, "light"],
  ["clair enregistré", { saved: "light" }, "light"],
  ["sombre enregistré", { saved: "dark" }, "dark"],
  ["préférence invalide", { saved: "system" }, "light"],
  ["lecture du stockage refusée", { failRead: true }, "light"],
]) {
  test(`thème avant montage : ${label}`, () => {
    const page = boot(options);
    assert.equal(page.theme(), expected);
    assert.equal(page.meta(), expected === "dark" ? "#111827" : "#F1E5D3");
    assert.equal(page.button.hidden, true);
    page.mounted();
    assert.equal(page.button.hidden, false);
    assert.equal(page.button.getAttribute("aria-pressed"), String(expected === "dark"));
  });
}
test("bascule et restauration de la préférence sans double abonnement", () => {
  const page = boot();
  page.mounted();
  page.mounted();
  page.button.click();
  assert.equal(page.theme(), "dark");
  assert.equal(page.button.getAttribute("aria-pressed"), "true");
  assert.equal(page.meta(), "#111827");
  assert.deepEqual(page.writes, [["filfinance.landing.theme", "dark"]]);
  assert.equal(boot({ saved: page.writes[0][1] }).theme(), "dark");
  page.button.click();
  assert.equal(page.theme(), "light");
  assert.equal(page.button.getAttribute("aria-pressed"), "false");
});
test("une écriture refusée conserve la bascule pour la visite", () => {
  const page = boot({ failWrite: true });
  page.mounted();
  assert.doesNotThrow(() => page.button.click());
  assert.equal(page.theme(), "dark");
  assert.equal(page.button.getAttribute("aria-pressed"), "true");
  assert.deepEqual(page.writes, []);
});

// Le contrôle de production ne doit pas pouvoir être contourné par un simple identifiant.
const inspect = (scripts) => validateScripts(parseHTML(`<!doctype html><html><head>${scripts}</head><body></body></html>`).document, source);
const trusted = `<script id="filfinance-theme">${source}</script>`;
test("script local exact et JSON-LD acceptés", () => {
  assert.deepEqual(inspect(`${trusted}<script type="application/ld+json">{}</script>`), []);
});
for (const [label, html] of [
  ["absent", ""],
  ["supplémentaire", `${trusted}<script>alert(1)</script>`],
  ["doublon", trusted + trusted],
  ["identifiant usurpé", '<script id="filfinance-theme">alert(1)</script>'],
  ["source modifiée", trusted.replace('let theme = "light"', 'let theme = "dark"')],
  ["source externe", '<script id="filfinance-theme" src="/evil.js"></script>'],
  ["attribut actif", trusted.replace('id="filfinance-theme"', 'id="filfinance-theme" onload="alert(1)"')],
]) {
  test(`script refusé : ${label}`, () => assert.ok(inspect(html).length > 0));
}
