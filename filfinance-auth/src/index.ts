import { createServer } from "node:http";
import { auth } from "./auth.js";
import { promoteInitialAdminIfReady } from "./bootstrap-admin.js";
import { appPool, authPool, closePools } from "./db.js";
import { env } from "./env.js";
import { applyCors, sendJson, sendWebResponse, toWebRequest } from "./http.js";
import { closeMailer } from "./mailer.js";
import { migrateDatabases } from "./migrations.js";
import { handleMe } from "./profiles.js";
import { withSafeAuthCallbacks } from "./verification-callback.js";

async function ready() {
  await Promise.all([authPool.query("SELECT 1"), appPool.query("SELECT 1")]);
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);

  try {
    if (applyCors(req, res)) {
      return;
    }

    if (url.pathname === "/health") {
      sendJson(res, 200, { ok: true, service: "filfinance-auth" });
      return;
    }

    if (url.pathname === "/ready") {
      await ready();
      sendJson(res, 200, { ok: true });
      return;
    }

    if (url.pathname.startsWith("/api/auth")) {
      const authRequest = withSafeAuthCallbacks(
        toWebRequest(req),
        env.APP_WEB_URL,
      );
      const response = await auth.handler(authRequest);
      await promoteInitialAdminIfReady();
      await sendWebResponse(res, response);
      return;
    }

    if (url.pathname === "/api/me") {
      const response = await handleMe(toWebRequest(req));
      await sendWebResponse(res, response);
      return;
    }

    sendJson(res, 404, { error: "not_found" });
  } catch {
    // SMTP client errors may carry provider diagnostics. Keep request logs
    // useful without serialising arbitrary errors or user-provided data.
    console.error("filfinance-auth request failed");
    sendJson(res, 500, { error: "internal_error" });
  }
});

await migrateDatabases();
await promoteInitialAdminIfReady();

server.listen(env.PORT, () => {
  console.log(`filfinance-auth listening on ${env.PORT}`);
});

async function shutdown(signal: string) {
  console.log(`Received ${signal}; shutting down filfinance-auth.`);
  server.close(async () => {
    await closeMailer();
    await closePools();
    process.exit(0);
  });
}

process.on("SIGTERM", () => void shutdown("SIGTERM"));
process.on("SIGINT", () => void shutdown("SIGINT"));
