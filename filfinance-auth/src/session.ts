import { auth } from "./auth.js";

export type AuthenticatedUser = {
  id: string;
  email: string;
  name: string | null;
  role: string;
};

export async function authenticateJwt(request: Request): Promise<AuthenticatedUser | null> {
  const token = bearerToken(request.headers.get("authorization"));
  if (!token) {
    return null;
  }

  const result = await auth.api.verifyJWT({
    body: { token }
  });
  return authenticatedUserFromJwtPayload(result.payload);
}

export function authenticatedUserFromJwtPayload(payload: unknown): AuthenticatedUser | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const claims = payload as Record<string, unknown>;
  if (
    typeof claims.sub !== "string" ||
    typeof claims.email !== "string" ||
    claims.email_verified !== true
  ) {
    return null;
  }

  return {
    id: claims.sub,
    email: claims.email,
    name: typeof claims.name === "string" ? claims.name : null,
    role: typeof claims.role === "string" ? claims.role : "listener"
  };
}

function bearerToken(header: string | null): string | null {
  if (!header) {
    return null;
  }
  const match = /^Bearer\s+(.+)$/i.exec(header.trim());
  const token = match?.[1]?.trim();
  return token || null;
}
