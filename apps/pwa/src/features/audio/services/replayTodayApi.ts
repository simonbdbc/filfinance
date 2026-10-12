import type { AuthSessionClient } from "@shared/api/authSession";
import type { HlsReplayTodayMetadata } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";

type LoadReplayTodayMetadataOptions = {
  replayTodayMetadataUrl: string;
  authSession: AuthSessionClient | null;
};

export async function loadReplayTodayMetadata({
  replayTodayMetadataUrl,
  authSession,
}: LoadReplayTodayMetadataOptions) {
  return cachedJsonGet<HlsReplayTodayMetadata>(replayTodayMetadataUrl, {
    authSession,
    context: "métadonnées de la rediffusion HLS du jour",
    ttlMs: 10_000,
  });
}
