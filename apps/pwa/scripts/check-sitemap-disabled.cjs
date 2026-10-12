const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const appConfigPath = path.join(projectRoot, "app.json");
const redirectsPath = path.join(projectRoot, "public", "_redirects");

const appConfig = JSON.parse(fs.readFileSync(appConfigPath, "utf8"));
const plugins = appConfig.expo?.plugins ?? [];
const expoRouterPlugin = plugins.find((plugin) => {
  if (plugin === "expo-router") {
    return true;
  }
  return Array.isArray(plugin) && plugin[0] === "expo-router";
});

if (!Array.isArray(expoRouterPlugin) || expoRouterPlugin[1]?.sitemap !== false) {
  throw new Error("expo-router sitemap must be disabled in app.json");
}

if (appConfig.expo?.extra?.router?.sitemap !== false) {
  throw new Error("expo.extra.router.sitemap must be false for web runtime builds");
}

const redirects = fs.readFileSync(redirectsPath, "utf8").trim().split(/\r?\n/u);

if (redirects[0] !== "/_sitemap /index.html 404") {
  throw new Error("public/_redirects must block /_sitemap before the SPA fallback");
}

if (redirects[1] !== "/_sitemap/* /index.html 404") {
  throw new Error("public/_redirects must block nested /_sitemap paths before the SPA fallback");
}

const distPath = path.join(projectRoot, "dist", "_expo", "static", "js", "web");
if (fs.existsSync(distPath)) {
  const entryFile = fs
    .readdirSync(distPath)
    .find((fileName) => fileName.startsWith("entry-") && fileName.endsWith(".js"));

  if (entryFile) {
    const entrySource = fs.readFileSync(path.join(distPath, entryFile), "utf8");
    const manifestMatch = entrySource.match(/get manifest\(\)\{return"((?:\\.|[^"])*)"\}/u);

    if (!manifestMatch) {
      throw new Error("exported web bundle manifest was not found");
    }

    const manifestJson = JSON.parse(manifestMatch[1].replace(/\\"/g, '"'));

    if (manifestJson.extra?.router?.sitemap !== false) {
      throw new Error("exported web bundle must include extra.router.sitemap=false");
    }
  }
}

console.log("[sitemap-disabled] expo-router sitemap and Cloudflare fallback guard OK");
