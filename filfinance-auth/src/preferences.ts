export type PreferencePatch = {
  preferred_tts_voice?: string | null;
  theme?: "light" | "dark" | null;
};

export async function parsePreferencePatch(request: Request): Promise<PreferencePatch | Response> {
  let input: unknown;
  try {
    input = await request.json();
  } catch {
    return Response.json({ error: "invalid_json" }, { status: 400 });
  }

  if (!input || typeof input !== "object" || Array.isArray(input)) {
    return Response.json({ error: "invalid_preferences" }, { status: 400 });
  }

  const record = input as Record<string, unknown>;
  const allowed = new Set(["preferred_tts_voice", "theme"]);
  if (Object.keys(record).some((key) => !allowed.has(key))) {
    return Response.json({ error: "unknown_preference" }, { status: 400 });
  }

  const patch: PreferencePatch = {};
  if (Object.hasOwn(record, "preferred_tts_voice")) {
    const voice = record.preferred_tts_voice;
    if (voice !== null && typeof voice !== "string") {
      return Response.json({ error: "invalid_preferred_tts_voice" }, { status: 400 });
    }
    const normalized = typeof voice === "string" ? voice.trim() : null;
    if (normalized && normalized.length > 128) {
      return Response.json({ error: "invalid_preferred_tts_voice" }, { status: 400 });
    }
    patch.preferred_tts_voice = normalized || null;
  }

  if (Object.hasOwn(record, "theme")) {
    const theme = record.theme;
    if (theme !== null && theme !== "light" && theme !== "dark") {
      return Response.json({ error: "invalid_theme" }, { status: 400 });
    }
    patch.theme = theme as PreferencePatch["theme"];
  }

  return patch;
}
