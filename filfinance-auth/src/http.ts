import type { IncomingMessage, ServerResponse } from "node:http";
import { Readable } from "node:stream";
import { env } from "./env.js";

export function sendJson(res: ServerResponse, status: number, payload: unknown): void {
  const body = JSON.stringify(payload);
  res.writeHead(status, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(body)
  });
  res.end(body);
}

export function applyCors(req: IncomingMessage, res: ServerResponse): boolean {
  const origin = req.headers.origin;
  if (origin && isTrustedHttpOrigin(origin)) {
    res.setHeader("access-control-allow-origin", origin);
    res.setHeader("access-control-allow-credentials", "true");
    res.setHeader("access-control-expose-headers", "set-auth-jwt, set-auth-token");
    res.setHeader("vary", "Origin");
  }

  if (req.method !== "OPTIONS") {
    return false;
  }

  if (!origin || !isTrustedHttpOrigin(origin)) {
    sendJson(res, 403, { error: "origin_not_allowed" });
    return true;
  }

  res.writeHead(204, {
    "access-control-allow-origin": origin,
    "access-control-allow-credentials": "true",
    "access-control-allow-methods": "GET, POST, PATCH, DELETE, OPTIONS",
    "access-control-allow-headers": req.headers["access-control-request-headers"] ?? "authorization, content-type",
    "access-control-max-age": "600",
    vary: "Origin"
  });
  res.end();
  return true;
}

function isTrustedHttpOrigin(origin: string): boolean {
  return env.TRUSTED_ORIGINS.some((pattern) => {
    if (!pattern.startsWith("http://") && !pattern.startsWith("https://")) {
      return false;
    }
    const escaped = pattern.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replaceAll("*", ".*");
    return new RegExp(`^${escaped}$`).test(origin);
  });
}

export function toWebRequest(req: IncomingMessage): Request {
  const headers = new Headers();
  for (const [key, value] of Object.entries(req.headers)) {
    if (Array.isArray(value)) {
      for (const item of value) {
        headers.append(key, item);
      }
    } else if (value != null) {
      headers.set(key, value);
    }
  }

  const protoHeader = headers.get("x-forwarded-proto");
  const hostHeader = headers.get("x-forwarded-host") ?? headers.get("host") ?? "localhost";
  const proto = protoHeader?.split(",")[0]?.trim() || "http";
  const url = new URL(req.url ?? "/", `${proto}://${hostHeader}`);
  const method = req.method ?? "GET";
  const hasBody = method !== "GET" && method !== "HEAD";

  return new Request(url, {
    method,
    headers,
    body: hasBody ? (Readable.toWeb(req) as ReadableStream) : undefined,
    duplex: hasBody ? "half" : undefined
  } as RequestInit & { duplex?: "half" });
}

export async function sendWebResponse(res: ServerResponse, response: Response): Promise<void> {
  res.statusCode = response.status;
  response.headers.forEach((value, key) => {
    if (key !== "set-cookie") {
      res.setHeader(key, value);
    }
  });
  const cookies = response.headers.getSetCookie();
  if (cookies.length > 0) {
    res.setHeader("set-cookie", cookies);
  }

  if (!response.body) {
    res.end();
    return;
  }

  await new Promise<void>((resolve, reject) => {
    Readable.fromWeb(response.body as import("node:stream/web").ReadableStream)
      .on("error", reject)
      .on("end", resolve)
      .pipe(res);
  });
}
