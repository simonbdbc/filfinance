import { authenticatedFetch } from "@shared/api/authSession";
import type { AuthSessionClient } from "@shared/api/authSession";
import { buildBearerHeaders } from "./authTransport";
import { throwHttpError } from "./httpErrors";

type CachedJsonGetOptions = {
  authSession: AuthSessionClient | null;
  context: string;
  signal?: AbortSignal;
  ttlMs?: number;
};

const DEFAULT_GET_CACHE_TTL_MS = 5_000;


export async function cachedJsonGet<T>(
  url: string,
  {
    authSession,
    context,
    signal,
    ttlMs = DEFAULT_GET_CACHE_TTL_MS,
  }: CachedJsonGetOptions,
): Promise<T> {
  const now = Date.now();
  const jsonGetCache = authSession?.cache;
  const key = url;
  for (const [entryKey, entry] of jsonGetCache ?? []) {
    if (entry.expiresAt <= now) jsonGetCache?.delete(entryKey);
  }
  const cached = jsonGetCache?.get(key);

  if (!signal && cached && cached.expiresAt > now) {
    return cached.promise as Promise<T>;
  }

  const promise = authenticatedFetch(authSession, url, {
    headers: buildBearerHeaders(authSession),
    signal,
  }).then(async (response) => {
    if (!response.ok) {
      await throwHttpError(response, context);
    }

    return (await response.json()) as T;
  });

  if (!signal) jsonGetCache?.set(key, {
    expiresAt: now + Math.max(0, ttlMs),
    promise,
  });

  promise.catch(() => {
    if (jsonGetCache?.get(key)?.promise === promise) {
      jsonGetCache?.delete(key);
    }
  });

  return promise;
}

export function invalidateCachedJsonGet(url: string, authSession: AuthSessionClient | null) {
  authSession?.cache.delete(url);
}
