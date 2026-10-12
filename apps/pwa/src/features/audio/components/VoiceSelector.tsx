import { View } from "react-native";

import { EyebrowText, SubtleText } from "@shared/ui/Typography";
import { Select } from "@shared/ui/Select";
import type { AudioTransportMode } from "@shared/api/contracts";
import {
  formatTtsVoiceDetailLabel,
  formatTtsVoiceOptionLabel,
} from "@shared/lib/ttsVoice";

type VoiceSelectorProps = {
  isAudioEnabled: boolean;
  audioTransportMode: AudioTransportMode;
  effectiveAudioVoice: string | null;
  availableVoices: string[];
  selectedVoice: string | null;
  pendingVoiceSelection: string | null;
  canChangeVoice: boolean;
  isUpdatingVoice: boolean;
  onVoiceChange: (voice: string) => void;
};

export function VoiceSelector({
  isAudioEnabled,
  audioTransportMode,
  effectiveAudioVoice,
  availableVoices,
  selectedVoice,
  pendingVoiceSelection,
  canChangeVoice,
  isUpdatingVoice,
  onVoiceChange,
}: VoiceSelectorProps) {
  const selectedFrenchVoice =
    selectedVoice && availableVoices.includes(selectedVoice)
      ? selectedVoice
      : "";
  const activeVoice =
    pendingVoiceSelection || selectedVoice || effectiveAudioVoice;
  const activeVoiceDetails = activeVoice
    ? formatTtsVoiceDetailLabel(activeVoice)
    : null;

  return (
    <View className="gap-3">
      <EyebrowText>Voix TTS serveur</EyebrowText>
      {!isAudioEnabled ? (
        <SubtleText>
          La voix du direct est définie par la configuration du serveur.
        </SubtleText>
      ) : audioTransportMode === "hls" ? (
        <>
          {activeVoiceDetails ? (
            <SubtleText>Voix active : {activeVoiceDetails}</SubtleText>
          ) : null}
          <SubtleText>
            Lecture HLS des extraits et des rediffusions activée. Le changement de voix à chaud par
            WebSocket audio a été retiré.
          </SubtleText>
        </>
      ) : availableVoices.length === 0 ? (
        <SubtleText>Chargement des voix françaises disponibles…</SubtleText>
      ) : (
        <>
          <Select
            label="Voix française"
            value={pendingVoiceSelection || selectedFrenchVoice}
            disabled={!canChangeVoice || isUpdatingVoice}
            onChange={(nextVoice) => {
              if (nextVoice) {
                onVoiceChange(nextVoice);
              }
            }}
            options={[
              {
                value: "",
                disabled: true,
                label: selectedFrenchVoice
                  ? "Sélectionnez une autre voix française"
                  : "Sélectionnez une voix française",
              },
              ...availableVoices.map((voice) => ({
                value: voice,
                label: formatTtsVoiceOptionLabel(voice),
              })),
            ]}
          />
          {activeVoiceDetails ? (
            <SubtleText>Voix active : {activeVoiceDetails}</SubtleText>
          ) : null}
          {selectedVoice && !availableVoices.includes(selectedVoice) ? (
            <SubtleText>
              Voix active hors liste française :{" "}
              {formatTtsVoiceDetailLabel(selectedVoice)}
            </SubtleText>
          ) : null}
          <SubtleText>
            {isUpdatingVoice
              ? "Application de la nouvelle voix sur le backend…"
              : "Seules les voix françaises sont proposées. Le changement est pris en compte sans redémarrer le service."}
          </SubtleText>
        </>
      )}
    </View>
  );
}
