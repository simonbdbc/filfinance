import { Switch } from "@shared/ui/Switch";
import { TextInput } from "@shared/ui/TextInput";
import { Text } from "@shared/ui/Text";
import { ActivityIndicator, View } from "react-native";

import { VoiceSelector } from "@features/audio";
import type {
  AudioPlaybackEngine,
  AudioTransportMode,
  ConnectionState,
  HlsPlaybackMode,
  HlsReplayTodayMetadata,
} from "@shared/api/contracts";
import { audioPlaybackEngineLabel, connectionLabel } from "@shared/lib/status";
import { useThemeTokens } from "@shared/theme";
import { formatTtsVoiceShortLabel } from "@shared/lib/ttsVoice";
import { StatusBadge } from "@shared/ui/StatusBadge";
import { Button } from "@shared/ui/Button";
import { Surface } from "@shared/ui/Surface";
import { EyebrowText, MetaText, SubtleText } from "@shared/ui/Typography";

type ConnectionSettingsCardProps = {
  connectionState: ConnectionState;
  wsUrlInput: string;
  onWsUrlInputChange: (value: string) => void;
  onReconnect: () => void;
  isWebPlatform: boolean;
  isAudioEnabled: boolean;
  onAudioEnabledChange: (value: boolean) => void;
  audioConnectionState: ConnectionState;
  lastAudioLaggedCount: number | null;
  bufferedAudioClipCount: number;
  effectiveAudioVoice: string | null;
  audioPlaybackEngine: AudioPlaybackEngine;
  audioTransportMode: AudioTransportMode;
  hlsPlaybackMode: HlsPlaybackMode;
  hlsReplayTodayMetadata: HlsReplayTodayMetadata | null;
  hlsAudioUrl: string | null;
  hlsReplayTodayPlaylistUrl: string;
  liveMetadataWsUrl: string;
  isLiveMetadataEdgeEnabled: boolean;
  isLiveMetadataFallbackActive: boolean;
  availableVoices: string[];
  selectedVoice: string | null;
  pendingVoiceSelection: string | null;
  canChangeVoice: boolean;
  isUpdatingVoice: boolean;
  onVoiceChange: (voice: string) => void;
};

export function ConnectionSettingsCard({
  connectionState,
  wsUrlInput,
  onWsUrlInputChange,
  onReconnect,
  isWebPlatform,
  isAudioEnabled,
  onAudioEnabledChange,
  audioConnectionState,
  lastAudioLaggedCount,
  bufferedAudioClipCount,
  effectiveAudioVoice,
  audioPlaybackEngine,
  audioTransportMode,
  hlsPlaybackMode,
  hlsReplayTodayMetadata,
  hlsAudioUrl,
  hlsReplayTodayPlaylistUrl,
  liveMetadataWsUrl,
  isLiveMetadataEdgeEnabled,
  isLiveMetadataFallbackActive,
  availableVoices,
  selectedVoice,
  pendingVoiceSelection,
  canChangeVoice,
  isUpdatingVoice,
  onVoiceChange,
}: ConnectionSettingsCardProps) {
  const tokens = useThemeTokens();
  const designTheme = useThemeTokens();
  const inputStyle = {
    backgroundColor: designTheme.colors.surface,
    borderColor: designTheme.colors.border,
    borderRadius: designTheme.radii.control,
    color: tokens.colors.textPrimary,
  };
  const isAudioToggleEnabled = isWebPlatform && isAudioEnabled;

  return (
    <Surface>
      <View className="w-full max-w-full flex-row flex-wrap items-center justify-between gap-3">
        <View className="min-w-0 flex-1">
          <EyebrowText>État du flux</EyebrowText>
          <Text
            className="mt-1 max-w-full min-w-0 text-xl font-extrabold"
            style={{ color: tokens.colors.textPrimary }}
          >
            {connectionLabel(connectionState)}
          </Text>
        </View>
        <StatusBadge
          variant={connectionState === "open" ? "dark" : "muted"}
          prefix={
            connectionState === "connecting" ? (
              <ActivityIndicator
                size="small"
                color={tokens.colors.textPrimary}
              />
            ) : null
          }
        >
          {connectionState === "open" ? "LIVE" : "WS"}
        </StatusBadge>
      </View>

      <EyebrowText className="mt-2">URL WebSocket</EyebrowText>
      <TextInput
        autoCapitalize="none"
        autoCorrect={false}
        keyboardType="url"
        value={wsUrlInput}
        onChangeText={onWsUrlInputChange}
        className="w-full max-w-full min-w-0 border px-4 py-3 text-sm"
        placeholder="ws://192.168.x.x:8088/ws/news"
        placeholderTextColor={tokens.colors.textPrimary}
        style={inputStyle}
      />
      <Button variant="primary" onPress={onReconnect}>
        Reconnecter avec cette URL
      </Button>

      <MetaText>
        Métadonnées du direct :{" "}
        {metadataModeLabel(
          isLiveMetadataEdgeEnabled,
          isLiveMetadataFallbackActive,
        )}
        {liveMetadataWsUrl ? ` • ${liveMetadataWsUrl}` : ""}
      </MetaText>

      <View className="mt-2 w-full max-w-full flex-row flex-wrap items-center justify-between gap-3">
        <View className="min-w-0 flex-1 gap-1">
          <EyebrowText>Lecture audio</EyebrowText>
          <SubtleText>
            {isWebPlatform
              ? "Direct événementiel par extraits HLS ; rediffusion du jour séparée."
              : "Lecture audio disponible sur le client Web uniquement."}
          </SubtleText>
        </View>
        <Switch
          accessibilityLabel="Activer le direct audio"
          disabled={!isWebPlatform}
          onValueChange={onAudioEnabledChange}
          value={isAudioToggleEnabled}
        />
      </View>

      {isWebPlatform ? (
        <>
          <MetaText>
            État audio : {connectionLabel(audioConnectionState)}
            {isAudioEnabled
              ? ` • transport=${audioTransportModeLabel(audioTransportMode)}`
              : ""}
            {audioTransportMode === "hls"
              ? ` • mode=${hlsPlaybackModeLabel(hlsPlaybackMode)}`
              : ""}
            {audioTransportMode === "hls"
              ? ` • player=${hlsPlayerStatusLabel(audioConnectionState)}`
              : ""}
            {lastAudioLaggedCount ? ` • lag=${lastAudioLaggedCount}` : ""}
            {bufferedAudioClipCount
              ? ` • buffer=${bufferedAudioClipCount}`
              : ""}
            {effectiveAudioVoice
              ? ` • voix=${formatTtsVoiceShortLabel(effectiveAudioVoice)}`
              : ""}
            {isAudioEnabled
              ? ` • moteur=${audioPlaybackEngineLabel(audioPlaybackEngine)}`
              : ""}
            {audioTransportMode === "hls" && hlsReplayTodayMetadata
              ? ` • replay=${hlsReplayTodayMetadata.segment_count} segments`
              : ""}
            {audioTransportMode === "hls" && hlsAudioUrl
              ? ` • ${hlsAudioUrl}`
              : ""}
          </MetaText>

          <MetaText>
            Extraits HLS via audio_ready • Rediffusion du jour : {hlsReplayTodayPlaylistUrl}
          </MetaText>

          <VoiceSelector
            isAudioEnabled={isAudioEnabled}
            audioTransportMode={audioTransportMode}
            effectiveAudioVoice={effectiveAudioVoice}
            availableVoices={availableVoices}
            selectedVoice={selectedVoice}
            pendingVoiceSelection={pendingVoiceSelection}
            canChangeVoice={canChangeVoice}
            isUpdatingVoice={isUpdatingVoice}
            onVoiceChange={onVoiceChange}
          />
        </>
      ) : null}
    </Surface>
  );
}

function metadataModeLabel(
  isLiveMetadataEdgeEnabled: boolean,
  isLiveMetadataFallbackActive: boolean,
) {
  if (!isLiveMetadataEdgeEnabled) {
    return "backend";
  }

  return isLiveMetadataFallbackActive ? "fallback backend" : "edge";
}

function audioTransportModeLabel(mode: AudioTransportMode) {
  switch (mode) {
    case "hls":
      return "HLS";
    default:
      return "idle";
  }
}

function hlsPlaybackModeLabel(mode: HlsPlaybackMode) {
  switch (mode) {
    case "clip":
      return "clip";
    case "replay_today":
      return "replay jour";
    case "daily_summary":
      return "résumé IA";
    case "calendar_summary":
      return "résumé calendrier";
    default:
      return "idle";
  }
}

function hlsPlayerStatusLabel(state: ConnectionState) {
  switch (state) {
    case "waiting":
      return "waiting";
    case "connecting":
      return "loading";
    case "open":
      return "playing";
    case "closed":
      return "delayed";
    case "error":
      return "error";
    default:
      return "idle";
  }
}
