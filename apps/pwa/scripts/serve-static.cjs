const { createReadStream, readFile, statSync } = require("node:fs");
const { createServer } = require("node:http");
const { extname, join, normalize, resolve } = require("node:path");

const root = resolve(process.argv[2] ?? "dist");
const port = Number.parseInt(process.argv[3] ?? "8052", 10);

const contentTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".ico", "image/x-icon"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".map", "application/json; charset=utf-8"],
  [".png", "image/png"],
  [".svg", "image/svg+xml"],
  [".txt", "text/plain; charset=utf-8"],
  [".webp", "image/webp"],
  [".woff", "font/woff"],
  [".woff2", "font/woff2"],
]);

function candidatePath(requestUrl) {
  const url = new URL(requestUrl, "http://localhost");
  const decodedPath = decodeURIComponent(url.pathname);
  const normalizedPath = normalize(decodedPath).replace(/^(\.\.[/\\])+/, "");
  const target = resolve(join(root, normalizedPath));

  if (!target.startsWith(root)) {
    return null;
  }

  return target;
}

function readableFile(pathname) {
  try {
    const stats = statSync(pathname);
    return stats.isFile() ? pathname : null;
  } catch {
    return null;
  }
}

function sendFile(response, pathname) {
  const extension = extname(pathname);
  response.writeHead(200, {
    "content-type": contentTypes.get(extension) ?? "application/octet-stream",
    "cache-control": extension === ".html" ? "no-cache" : "public, max-age=31536000, immutable",
  });
  createReadStream(pathname).pipe(response);
}

const runtimeEnvKeys = [
  "EXPO_PUBLIC_LANDING_ORIGIN",
  "EXPO_PUBLIC_FILFINANCE_WS_URL",
  "EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL",
  "EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL",
  "EXPO_PUBLIC_FILFINANCE_PWA_URL",
  "EXPO_PUBLIC_API_URL",
  "EXPO_PUBLIC_BETTER_AUTH_URL",
];

function runtimeEnvScript() {
  const env = Object.fromEntries(
    runtimeEnvKeys
      .map((key) => [key, process.env[key]?.trim() ?? ""])
      .filter(([, value]) => value.length > 0),
  );

  return `<script>window.__FILFINANCE_ENV__=${JSON.stringify(env)};</script>`;
}

function sendIndex(response, pathname) {
  readFile(pathname, "utf8", (error, html) => {
    if (error) {
      response.writeHead(500, { "content-type": "text/plain; charset=utf-8" });
      response.end("Unable to read index.html");
      return;
    }

    response.writeHead(200, {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-cache",
    });
    response.end(html.replace("</head>", `${runtimeEnvScript()}</head>`));
  });
}

createServer((request, response) => {
  const target = candidatePath(request.url ?? "/");
  const indexPath = resolve(join(root, "index.html"));

  if (target === null) {
    response.writeHead(400).end("Bad request");
    return;
  }

  const file = readableFile(target) ?? readableFile(join(target, "index.html"));
  const responseFile = file ?? indexPath;

  if (responseFile === indexPath) {
    sendIndex(response, indexPath);
    return;
  }

  sendFile(response, responseFile);
}).listen(port, "0.0.0.0", () => {
  console.log(`filfinance-client listening on http://0.0.0.0:${port}`);
});
