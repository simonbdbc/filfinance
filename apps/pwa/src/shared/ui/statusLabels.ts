import type { Tone } from "./workspaceTypes";
export function badgeVariantForTone(
  tone: Tone = "muted",
): "critical" | "dark" | "info" | "muted" | "trace" {
  return tone;
}

export function toneLabel(tone: Tone = "muted") {
  if (tone === "critical") {
    return "alerte";
  }
  if (tone === "dark") {
    return "action";
  }
  if (tone === "info") {
    return "ok";
  }
  if (tone === "trace") {
    return "suivi";
  }
  return "attente";
}
