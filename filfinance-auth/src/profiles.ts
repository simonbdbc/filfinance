import { appPool } from "./db.js";
import { parsePreferencePatch, type PreferencePatch } from "./preferences.js";
import { authenticateJwt, type AuthenticatedUser } from "./session.js";

type ProfileRow = {
  id: string;
  email: string;
  preferred_tts_voice: string | null;
  theme: "light" | "dark" | null;
  created_at: Date;
  updated_at: Date;
};

export async function handleMe(request: Request): Promise<Response> {
  const user = await authenticateJwt(request).catch(() => null);
  if (!user) {
    return jsonResponse(401, { error: "unauthorized" });
  }

  if (request.method === "GET") {
    const profile = await upsertProfile(user);
    return jsonResponse(200, publicProfile(profile, user));
  }

  if (request.method === "PATCH") {
    const patch = await parsePreferencePatch(request);
    if (patch instanceof Response) {
      return patch;
    }
    const profile = await updatePreferences(user, patch);
    return jsonResponse(200, publicProfile(profile, user));
  }

  return jsonResponse(405, { error: "method_not_allowed" }, { allow: "GET, PATCH" });
}

async function upsertProfile(user: AuthenticatedUser): Promise<ProfileRow> {
  const result = await appPool.query<ProfileRow>(
    `
      INSERT INTO app.profiles (auth_user_id, email, display_name)
      VALUES ($1, $2, $3)
      ON CONFLICT (auth_user_id) DO UPDATE SET
        email = EXCLUDED.email,
        display_name = COALESCE(EXCLUDED.display_name, app.profiles.display_name),
        updated_at = now()
      RETURNING id, email, preferred_tts_voice, theme, created_at, updated_at
    `,
    [user.id, user.email, user.name]
  );
  return result.rows[0];
}

async function updatePreferences(
  user: AuthenticatedUser,
  patch: PreferencePatch
): Promise<ProfileRow> {
  await upsertProfile(user);
  const hasVoice = Object.hasOwn(patch, "preferred_tts_voice");
  const hasTheme = Object.hasOwn(patch, "theme");
  const result = await appPool.query<ProfileRow>(
    `
      UPDATE app.profiles
      SET
        preferred_tts_voice = CASE WHEN $1 THEN $2 ELSE preferred_tts_voice END,
        theme = CASE WHEN $3 THEN $4 ELSE theme END,
        updated_at = now()
      WHERE auth_user_id = $5
      RETURNING id, email, preferred_tts_voice, theme, created_at, updated_at
    `,
    [hasVoice, patch.preferred_tts_voice ?? null, hasTheme, patch.theme ?? null, user.id]
  );
  return result.rows[0];
}

function publicProfile(profile: ProfileRow, user: AuthenticatedUser) {
  return {
    id: profile.id,
    email: profile.email,
    role: user.role,
    preferences: {
      preferred_tts_voice: profile.preferred_tts_voice,
      theme: profile.theme
    },
    created_at: profile.created_at.toISOString(),
    updated_at: profile.updated_at.toISOString()
  };
}

function jsonResponse(status: number, payload: unknown, headers?: HeadersInit): Response {
  return Response.json(payload, { status, headers });
}
