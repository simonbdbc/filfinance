import { View, useWindowDimensions } from "react-native";
import type {
  ConnectionState,
  AudioTransportMode,
  HlsPlaybackMode,
} from "../api/contracts";
import { Text } from "./Text";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { useThemeTokens } from "../theme/theme";
export type AudioControl = {
  isWebPlatform: boolean;
  isAudioEnabled: boolean;
  audioConnectionState: ConnectionState;
  audioTransportMode: AudioTransportMode;
  hlsPlaybackMode: HlsPlaybackMode;
  bufferedAudioClipCount: number;
  dailySummaryPlaybackState: "idle" | "waiting" | "playing";
  dailySummaryAvailabilityStatus: "loading" | "ready" | "unavailable" | "error";
  isDailyAudioReplayActive: boolean;
  onAudioEnabledChange: (value: boolean) => void;
  onDailySummaryReplayPress: () => void;
};
export function summaryAction(control: AudioControl) {
  if (control.dailySummaryPlaybackState === "waiting")
    return "Arrêter la préparation";
  if (
    control.dailySummaryPlaybackState === "playing" ||
    control.isDailyAudioReplayActive
  )
    return "Arrêter la lecture";
  if (control.dailySummaryAvailabilityStatus === "loading")
    return "Résumé en préparation";
  if (control.dailySummaryAvailabilityStatus !== "ready")
    return "Résumé indisponible";
  return "Résumé audio du jour";
}
export function AudioPlayerView({ control }: { control: AudioControl }) {
  const t = useThemeTokens();
  const compact = useWindowDimensions().width < 768;
  const runningSummary =
    control.isDailyAudioReplayActive ||
    control.dailySummaryPlaybackState !== "idle";
  const status = !control.isWebPlatform
    ? "Lecture disponible sur le Web"
    : control.audioConnectionState === "error"
      ? "Connexion audio interrompue"
      : !control.isAudioEnabled
        ? "À votre écoute"
        : control.audioConnectionState === "open"
          ? "Direct activé"
          : "Connexion au direct…";
  return (
    <View
      testID="session-audio"
      className="w-full min-w-0 flex-row flex-wrap items-center gap-4 p-4 md:p-6 mb-2"
      style={{ backgroundColor: t.colors.surface, borderRadius: t.radii.panel }}
    >
      {compact ? (
        <View className="w-full flex-row items-center gap-3">
          <Button
            variant="primary"
            accessibilityLabel={
              control.isAudioEnabled ? "Arrêter le direct" : "Écouter le direct"
            }
            disabled={!control.isWebPlatform}
            onPress={() =>
              control.onAudioEnabledChange(!control.isAudioEnabled)
            }
            prefix={
              <Icon
                name={control.isAudioEnabled ? "square" : "play"}
                color={t.colors.primaryText}
              />
            }
          />
          <View className="min-w-0 flex-1 gap-1">
            <Text style={{ fontWeight: "600" }}>
              {control.isAudioEnabled ? "Direct activé" : "Écouter le direct"}
            </Text>
            <Text style={{ fontSize: 14 }}>Flux audio · Français</Text>
            <Text accessibilityLiveRegion="polite" style={{ fontSize: 14 }}>
              {status}
            </Text>
          </View>
        </View>
      ) : (
        <>
          <Button
            variant="primary"
            disabled={!control.isWebPlatform}
            onPress={() =>
              control.onAudioEnabledChange(!control.isAudioEnabled)
            }
            prefix={
              <Icon
                name={control.isAudioEnabled ? "square" : "play"}
                color={t.colors.primaryText}
              />
            }
          >
            {control.isAudioEnabled ? "Arrêter le direct" : "Écouter le direct"}
          </Button>
          <View className="min-w-0 flex-1 gap-1">
            <Text style={{ fontSize: 14 }}>Flux audio · Français</Text>
            <Text accessibilityLiveRegion="polite" style={{ fontSize: 14 }}>
              {status}
            </Text>
          </View>
        </>
      )}
      <Button
        disabled={
          !runningSummary && control.dailySummaryAvailabilityStatus !== "ready"
        }
        onPress={control.onDailySummaryReplayPress}
        prefix={
          <Icon name={runningSummary ? "square" : "headphones"} size={18} />
        }
      >
        {summaryAction(control)}
      </Button>
    </View>
  );
}
