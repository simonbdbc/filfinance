import type { AuthSessionClient } from "@shared/api/authSession";
import { useCallback, useEffect, useRef, useState } from "react";

import { appendHlsAccessTicket, loadHlsAccessTicket } from "../services/hlsAccessTicketApi";
import { loadReplayTodayMetadata } from "../services/replayTodayApi";
import type {
  AudioPlaybackEngine,
  ConnectionState,
  HlsPlaybackMode,
  HlsReplayTodayMetadata,
} from "@shared/api/contracts";
import { resolveHlsResourceUrl } from "@shared/api/urls";

const HLS_RETRY_DELAY_MS = 3_000;
const HLS_MAX_RETRY_DELAY_MS = 15_000;
const REPLAY_TODAY_UNAVAILABLE_MESSAGE =
  "Le résumé audio du jour n’est pas encore disponible : aucun segment audio n’a été généré pour les actualités visibles.";

type HlsAudioPlayerOptions = {
  isWebPlatform: boolean;
  wsUrl: string;
  replayTodayPlaylistUrl: string;
  replayTodayMetadataUrl: string;
  hlsAccessTicketUrl: string;
  authSession: AuthSessionClient | null;
  onPlaybackError: (message: string) => void;
  onClipEnded?: () => void;
  onReplayTodayEnded?: () => void;
};

type HlsErrorData = {
  fatal?: boolean;
  type?: string;
  details?: string;
  reason?: string;
  response?: {
    code?: number;
    text?: string;
  };
};

type HlsEventCallback = (event: string, data?: HlsErrorData) => void;

type CachedHlsAccessTicket = {
  ticket: string;
  expiresAtMs: number;
};

type HlsJsInstance = {
  attachMedia: (media: HTMLMediaElement) => void;
  loadSource: (source: string) => void;
  destroy: () => void;
  on: (event: string, callback: HlsEventCallback) => void;
};

type HlsJsConstructor = {
  new (config?: Record<string, unknown>): HlsJsInstance;
  isSupported: () => boolean;
  Events: {
    ERROR: string;
    MANIFEST_PARSED: string;
  };
};

export function useHlsAudioPlayer({
  isWebPlatform,
  wsUrl,
  replayTodayPlaylistUrl,
  replayTodayMetadataUrl,
  hlsAccessTicketUrl,
  authSession,
  onPlaybackError,
  onClipEnded,
  onReplayTodayEnded,
}: HlsAudioPlayerOptions) {
  const [hlsConnectionState, setHlsConnectionState] =
    useState<ConnectionState>("idle");
  const [hlsPlaybackMode, setHlsPlaybackMode] =
    useState<HlsPlaybackMode>("idle");
  const [hlsReplayTodayMetadata, setHlsReplayTodayMetadata] =
    useState<HlsReplayTodayMetadata | null>(null);
  const [hlsAudioUrl, setHlsAudioUrl] = useState<string | null>(null);
  const [hlsPlaybackEngine, setHlsPlaybackEngine] =
    useState<AudioPlaybackEngine>("unknown");
  const hlsInstanceRef = useRef<HlsJsInstance | null>(null);
  const hlsAudioElementRef = useRef<HTMLAudioElement | null>(null);
  const hlsAccessTicketRef = useRef<CachedHlsAccessTicket | null>(null);

  const stopHlsPlayback = (shouldUpdateState = true) => {
    hlsInstanceRef.current?.destroy();
    hlsInstanceRef.current = null;

    if (hlsAudioElementRef.current) {
      const audioElement = hlsAudioElementRef.current;
      audioElement.pause();
      audioElement.removeAttribute("src");
      audioElement.load();
      audioElement.remove();
      hlsAudioElementRef.current = null;
    }

    if (!shouldUpdateState) {
      return;
    }

    setHlsConnectionState("idle");
    setHlsPlaybackMode("idle");
    setHlsAudioUrl(null);
    setHlsPlaybackEngine("unknown");
  };

  const refreshReplayTodayMetadata = useCallback(async () => {
    if (!isWebPlatform || !authSession) {
      setHlsReplayTodayMetadata(null);
      return null;
    }

    const metadata = await loadReplayTodayMetadata({
      replayTodayMetadataUrl,
      authSession,
    });
    setHlsReplayTodayMetadata(metadata);
    return metadata;
  }, [authSession, isWebPlatform, replayTodayMetadataUrl]);

  const startReplayTodayHlsPlayback = async (startTranslationId?: number | null) => {
    const metadata = await refreshReplayTodayMetadata();

    if (!metadata) {
      throw new Error("Métadonnées de la rediffusion HLS du jour indisponibles");
    }

    if (!metadata.enabled) {
      throw new Error("Rediffusion HLS du jour désactivée côté serveur");
    }

    if (!metadata.available) {
      throw new Error(REPLAY_TODAY_UNAVAILABLE_MESSAGE);
    }

    const playlistUrl = metadata.playlist_url
      ? resolveHlsResourceUrl(wsUrl, metadata.playlist_url)
      : replayTodayPlaylistUrl;
    await startHlsPlayback(
      appendReplayStartTranslationId(playlistUrl, startTranslationId),
      "replay_today",
    );

    return metadata;
  };

  const startClipHlsPlayback = async (playlistUrl: string) => {
    const sourceUrl = resolveHlsResourceUrl(wsUrl, playlistUrl);
    await startHlsPlayback(sourceUrl, "clip");
  };

  const startDailySummaryHlsPlayback = async (playlistUrl: string) => {
    const sourceUrl = resolveHlsResourceUrl(wsUrl, playlistUrl);
    await startHlsPlayback(sourceUrl, "calendar_summary");
  };

  const isHlsPlaybackActive = () => {
    return (
      hlsConnectionState === "open"
      || hlsConnectionState === "connecting"
      || hlsConnectionState === "waiting"
    );
  };

  useEffect(() => {
    return () => {
      stopHlsPlayback(false);
    };
  }, []);

  useEffect(() => {
    hlsAccessTicketRef.current = null;
    setHlsReplayTodayMetadata(null);
  }, [authSession, hlsAccessTicketUrl, replayTodayMetadataUrl]);

  async function startHlsPlayback(sourceUrl: string, playbackMode: HlsPlaybackMode) {
    if (!isWebPlatform || typeof document === "undefined") {
      throw new Error("Lecture HLS disponible sur le client Web uniquement");
    }

    stopHlsPlayback();
    setHlsConnectionState("connecting");
    setHlsPlaybackMode(playbackMode);
    setHlsAudioUrl(sourceUrl);

    const audioElement = createHeadlessAudioElement(playbackMode);
    audioElement.addEventListener("ended", () => {
      if (hlsAudioElementRef.current !== audioElement) {
        return;
      }

      if (playbackMode === "clip") {
        onClipEnded?.();
        return;
      }

      // Le replay de jour est une lecture ponctuelle: son etat UI doit retomber
      // en idle quand la playlist est terminee.
      stopHlsPlayback();
      onReplayTodayEnded?.();
    });
    hlsAudioElementRef.current = audioElement;

    try {
      const securedSourceUrl = await resolveSecuredHlsSourceUrl(sourceUrl);
      const Hls = await loadHlsJsConstructor().catch(() => null);
      if (Hls?.isSupported()) {
        setHlsPlaybackEngine("hls_js");
        const hls = new Hls({
          enableWorker: true,
          liveDurationInfinity: playbackMode === "clip",
          liveSyncDurationCount: 3,
          lowLatencyMode: false,
          manifestLoadingMaxRetry: 0,
          manifestLoadPolicy: hlsBoundedLoadPolicy(),
          playlistLoadPolicy: hlsBoundedLoadPolicy(),
          // Les clips courts doivent partir du debut; le live-edge peut sauter
          // l'intro ou bloquer sur un manifest ouvert avant finalisation.
          startPosition: 0,
        });
        hlsInstanceRef.current = hls;

        await waitForHlsManifest({
          Hls,
          hls,
          sourceUrl: securedSourceUrl,
          audioElement,
        });
        hls.on(Hls.Events.ERROR, (_event, data) => {
          if (!data?.fatal || hlsInstanceRef.current !== hls) {
            return;
          }

          setHlsConnectionState("error");
          onPlaybackError(formatHlsError(data));
          if (playbackMode === "clip") {
            onClipEnded?.();
          }
        });
        await audioElement.play();
        setHlsConnectionState("open");
        return;
      }

      if (canPlayNativeHls(audioElement)) {
        setHlsPlaybackEngine("hls_native");
        audioElement.addEventListener("error", () => {
          if (hlsAudioElementRef.current !== audioElement) {
            return;
          }

          setHlsConnectionState("error");
          onPlaybackError(formatNativeHlsError(audioElement));
          if (playbackMode === "clip") {
            onClipEnded?.();
          }
        });
        audioElement.src = securedSourceUrl;
        audioElement.load();
        await audioElement.play();
        setHlsConnectionState("open");
        return;
      }

      throw new Error("HLS non pris en charge par ce navigateur");
    } catch (error) {
      const message = errorMessage(error);
      stopHlsPlayback();
      setHlsConnectionState("error");
      onPlaybackError(message);
      throw error;
    }
  }

  async function resolveSecuredHlsSourceUrl(sourceUrl: string) {
    const ticket = await getHlsAccessTicket();
    return ticket ? appendHlsAccessTicket(sourceUrl, ticket) : sourceUrl;
  }

  async function getHlsAccessTicket() {
    if (!authSession) {
      return null;
    }

    const cachedTicket = hlsAccessTicketRef.current;
    if (cachedTicket && cachedTicket.expiresAtMs - Date.now() > 60_000) {
      return cachedTicket.ticket;
    }

    const response = await loadHlsAccessTicket({
      ticketUrl: hlsAccessTicketUrl,
      authSession,
    });

    if (!response.enabled || !response.ticket) {
      hlsAccessTicketRef.current = null;
      return null;
    }

    const expiresAtMs = parseHlsTicketExpiration(response);
    hlsAccessTicketRef.current = {
      ticket: response.ticket,
      expiresAtMs,
    };

    return response.ticket;
  }

  return {
    hlsConnectionState,
    hlsPlaybackMode,
    hlsReplayTodayMetadata,
    hlsAudioUrl,
    hlsPlaybackEngine,
    isHlsPlaybackActive,
    refreshReplayTodayMetadata,
    startClipHlsPlayback,
    startDailySummaryHlsPlayback,
    startReplayTodayHlsPlayback,
    stopHlsPlayback,
  };
}

function canPlayNativeHls(audioElement: HTMLAudioElement) {
  return Boolean(
    audioElement.canPlayType("application/vnd.apple.mpegurl")
    || audioElement.canPlayType("application/x-mpegURL"),
  );
}

function appendReplayStartTranslationId(
  playlistUrl: string,
  startTranslationId?: number | null,
) {
  if (!startTranslationId || startTranslationId <= 0) {
    return playlistUrl;
  }

  try {
    const parsed = new URL(playlistUrl);
    parsed.searchParams.set("start_translation_id", String(startTranslationId));
    return parsed.toString();
  } catch {
    const separator = playlistUrl.includes("?") ? "&" : "?";
    return `${playlistUrl}${separator}start_translation_id=${encodeURIComponent(
      String(startTranslationId),
    )}`;
  }
}

function createHeadlessAudioElement(playbackMode: HlsPlaybackMode) {
  const audioElement = document.createElement("audio");
  audioElement.autoplay = true;
  audioElement.controls = false;
  audioElement.disableRemotePlayback = true;
  audioElement.preload = isLongFormPlayback(playbackMode) ? "metadata" : "auto";
  audioElement.tabIndex = -1;
  audioElement.setAttribute("aria-hidden", "true");
  audioElement.setAttribute("controlsList", "nodownload noplaybackrate noremoteplayback");
  audioElement.setAttribute("playsinline", "true");
  return audioElement;
}

function isLongFormPlayback(playbackMode: HlsPlaybackMode) {
  return (
    playbackMode === "replay_today" ||
    playbackMode === "daily_summary" ||
    playbackMode === "calendar_summary"
  );
}

async function loadHlsJsConstructor() {
  const module = await import("hls.js");
  return (module.default ?? module) as HlsJsConstructor;
}

function hlsBoundedLoadPolicy() {
  return {
    default: {
      maxTimeToFirstByteMs: 10_000,
      maxLoadTimeMs: 20_000,
      timeoutRetry: {
        maxNumRetry: 1,
        retryDelayMs: HLS_RETRY_DELAY_MS,
        maxRetryDelayMs: HLS_MAX_RETRY_DELAY_MS,
      },
      errorRetry: {
        maxNumRetry: 1,
        retryDelayMs: HLS_RETRY_DELAY_MS,
        maxRetryDelayMs: HLS_MAX_RETRY_DELAY_MS,
      },
    },
  };
}

function parseHlsTicketExpiration(response: {
  expires_at: string | null;
  ttl_secs: number | null;
}) {
  const parsedExpiresAt = response.expires_at ? Date.parse(response.expires_at) : Number.NaN;
  if (Number.isFinite(parsedExpiresAt)) {
    return parsedExpiresAt;
  }

  const ttlMs = typeof response.ttl_secs === "number" ? response.ttl_secs * 1000 : 0;
  return Date.now() + Math.max(ttlMs, 60_000);
}

function waitForHlsManifest({
  Hls,
  hls,
  sourceUrl,
  audioElement,
}: {
  Hls: HlsJsConstructor;
  hls: HlsJsInstance;
  sourceUrl: string;
  audioElement: HTMLAudioElement;
}) {
  return new Promise<void>((resolve, reject) => {
    let settled = false;
    const timeout = setTimeout(() => {
      if (!settled) {
        settled = true;
        resolve();
      }
    }, 5000);

    const settleResolve = () => {
      if (settled) {
        return;
      }

      settled = true;
      clearTimeout(timeout);
      resolve();
    };
    const settleReject = (error: Error) => {
      if (settled) {
        return;
      }

      settled = true;
      clearTimeout(timeout);
      reject(error);
    };

    hls.on(Hls.Events.MANIFEST_PARSED, settleResolve);
    hls.on(Hls.Events.ERROR, (_event, data) => {
      if (data?.fatal) {
        settleReject(new Error(formatHlsError(data)));
      }
    });
    hls.attachMedia(audioElement);
    hls.loadSource(sourceUrl);
  });
}

function formatHlsError(data: HlsErrorData) {
  const parts = [
    data.type,
    data.details,
    data.reason,
    data.response?.code ? `HTTP ${data.response.code}` : null,
    data.response?.text,
  ].filter(Boolean);

  return parts.length ? `Erreur HLS : ${parts.join(" - ")}` : "Erreur HLS";
}

function formatNativeHlsError(audioElement: HTMLAudioElement) {
  const mediaError = audioElement.error;
  if (!mediaError) {
    return "Erreur HLS native";
  }

  const labels: Record<number, string> = {
    1: "lecture interrompue",
    2: "erreur réseau",
    3: "erreur du décodeur",
    4: "source non prise en charge",
  };
  const label = labels[mediaError.code] ?? `code ${mediaError.code}`;
  return `Erreur HLS native : ${label}`;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Erreur HLS inconnue";
}
