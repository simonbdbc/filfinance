import { Text } from "@shared/ui/Text";
import { View } from "react-native";

import { SectionPanel, StateIndicator } from "@shared/ui/Workspace";

import type { TranslatedNewsItem } from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import { itemTimestampValue } from "@shared/lib/newsHistory";
import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { EyebrowText, MetaText, SubtleText } from "@shared/ui/Typography";

type CurrentLiveNewsCardProps = {
  currentItem: TranslatedNewsItem | null;
  currentOrLatestItem: TranslatedNewsItem | null;
  onStop: () => void;
  onClearQueue: () => void;
};

export function CurrentLiveNewsCard({
  currentItem,
  currentOrLatestItem,
  onStop,
  onClearQueue,
}: CurrentLiveNewsCardProps) {
  const tokens = useThemeTokens();
  const isCriticalLevel =
    currentOrLatestItem?.news_level === "active active-critical";

  return (
    <SectionPanel
      title={currentItem ? "Actualité en lecture" : "Dernière actualité reçue"}
      description="Dernier contenu transmis par le service en direct."
      action={
        currentOrLatestItem?.news_level ? (
          <StateIndicator tone={isCriticalLevel ? "critical" : "info"}>
            {currentOrLatestItem.news_level}
          </StateIndicator>
        ) : null
      }
    >
      <EyebrowText
        className="text-sm tracking-[0.8px]"
        style={{ color: tokens.colors.textPrimary }}
      >
        Source EN
      </EyebrowText>
      <Text
        className="max-w-full min-w-0 break-words text-[15px] font-semibold leading-[21px]"
        style={{ color: tokens.colors.textSecondary }}
      >
        {currentOrLatestItem?.source_title || "Aucune actualité source reçue"}
      </Text>
      <EyebrowText className="mt-2">Traduction FR</EyebrowText>
      <Text
        className="max-w-full min-w-0 break-words text-lg font-extrabold leading-6"
        style={{ color: tokens.colors.textPrimary }}
      >
        {currentOrLatestItem?.translated_title || "Aucune actualité traduite reçue"}
      </Text>
      <MetaText
        className="leading-none"
        style={{ color: tokens.colors.textPrimary }}
      >
        {currentOrLatestItem
          ? formatLocalDateTimeWithRaw(
              itemTimestampValue(currentOrLatestItem),
              "timestamp inconnu",
            )
          : "timestamp inconnu"}
        {currentOrLatestItem?.news_level
          ? ` • level=${currentOrLatestItem.news_level}`
          : ""}
        {currentOrLatestItem?.url ? ` • ${currentOrLatestItem.url}` : ""}
        {currentOrLatestItem?.test_run_id
          ? ` • test_run=${currentOrLatestItem.test_run_id}`
          : ""}
      </MetaText>
      <SubtleText>
        {currentOrLatestItem?.narration_text ||
          "Le flux reste en attente de `translated_news_item`."}
      </SubtleText>

      <View className="mt-1 w-full max-w-full flex-row flex-wrap gap-3">
        <Button
          className="min-w-[120px] flex-1"
          disabled={!currentItem}
          onPress={onStop}
        >
          Arrêter
        </Button>
        <Button className="min-w-[120px] flex-1" onPress={onClearQueue}>
          Vider la file
        </Button>
      </View>
    </SectionPanel>
  );
}
