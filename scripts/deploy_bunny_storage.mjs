#!/usr/bin/env node
/**
 * Synchronise une Storage Zone Bunny sans rendre un nouveau build partiellement visible.
 * Les assets sont envoyés avant index.html, puis les objets devenus orphelins sont supprimés à
 * partir du manifeste distant. Aucun secret n'est écrit dans les artefacts.
 */
import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

const args = new Map(process.argv.slice(2).map((value, index, values) =>
  value.startsWith("--") ? [value.slice(2), values[index + 1]] : []
).filter(([key]) => key));
const dist = path.resolve(args.get("dist") || process.env.BUNNY_STORAGE_DIST || "");
const zone = process.env.BUNNY_STORAGE_ZONE;
const password = process.env.BUNNY_STORAGE_PASSWORD;
const pullZoneId = process.env.BUNNY_PULL_ZONE_ID;
// La cle est fournie par le job appelant. Le script est ainsi partage entre
// DEV et PROD sans connaitre ni afficher le nom d'une variable d'environnement
// specifique a un environnement.
const apiKey = process.env.BUNNY_API_KEY;
const storageBase = (process.env.BUNNY_STORAGE_ENDPOINT || "https://storage.bunnycdn.com").replace(/\/$/, "");
const apiBase = (process.env.BUNNY_API_ENDPOINT || "https://api.bunny.net").replace(/\/$/, "");
const manifestPath = ".filfinance-bunny-manifest.json";

for (const [name, value] of Object.entries({ dist, zone, password, pullZoneId, apiKey })) {
  if (!value) {
    const variable = {
      password: "BUNNY_STORAGE_PASSWORD",
      pullZoneId: "BUNNY_PULL_ZONE_ID",
      apiKey: "BUNNY_API_KEY",
      zone: "BUNNY_STORAGE_ZONE",
      dist: "BUNNY_STORAGE_DIST or --dist",
    }[name];
    throw new Error(`Missing ${variable}`);
  }
}

async function listFiles(directory, prefix = "") {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const relative = path.posix.join(prefix, entry.name);
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await listFiles(absolute, relative));
    else if (entry.isFile() && relative !== manifestPath) files.push(relative);
  }
  return files.sort();
}

function objectUrl(relative) {
  const encoded = relative.split("/").map(encodeURIComponent).join("/");
  return `${storageBase}/${encodeURIComponent(zone)}/${encoded}`;
}

async function storageRequest(relative, init = {}) {
  const response = await fetch(objectUrl(relative), {
    ...init,
    headers: { AccessKey: password, ...(init.headers || {}) },
  });
  if (!response.ok && response.status !== 404) {
    throw new Error(`Bunny Storage ${init.method || "GET"} ${relative} failed (${response.status})`);
  }
  return response;
}

async function remoteManifest() {
  const response = await storageRequest(manifestPath);
  if (response.status === 404) return { files: [] };
  try {
    const manifest = await response.json();
    return Array.isArray(manifest.files) ? manifest : { files: [] };
  } catch {
    // Une ancienne version ou un objet manuel ne donne jamais le droit de supprimer à l'aveugle.
    return { files: [] };
  }
}

const files = await listFiles(dist);
const previous = await remoteManifest();
const manifest = {
  version: 1,
  files: await Promise.all(files.map(async relative => {
    const body = await readFile(path.join(dist, relative));
    return { path: relative, sha256: createHash("sha256").update(body).digest("hex") };
  })),
};

// index.html est toujours publié après les bundles hashés afin d'éviter les références vers un
// build encore absent chez Bunny.
const orderedFiles = files.filter(file => file !== "index.html").concat(files.includes("index.html") ? ["index.html"] : []);
for (const relative of orderedFiles) {
  const body = await readFile(path.join(dist, relative));
  await storageRequest(relative, { method: "PUT", body });
}

const currentPaths = new Set(files);
for (const item of previous.files) {
  if (typeof item?.path === "string" && item.path !== manifestPath && !currentPaths.has(item.path)) {
    await storageRequest(item.path, { method: "DELETE" });
  }
}
await storageRequest(manifestPath, {
  method: "PUT",
  body: JSON.stringify(manifest),
  headers: { "Content-Type": "application/json" },
});

const purge = await fetch(`${apiBase}/pullzone/${encodeURIComponent(pullZoneId)}/purgeCache`, {
  method: "POST",
  headers: { AccessKey: apiKey },
});
if (!purge.ok) throw new Error(`Bunny Pull Zone purge failed (${purge.status})`);
console.log(`Bunny Storage synchronized: ${files.length} files, Pull Zone ${pullZoneId} purged.`);
