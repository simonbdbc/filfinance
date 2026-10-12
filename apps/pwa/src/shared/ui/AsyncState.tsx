import { ActivityIndicator, View } from "react-native";
import { Text } from "./Text";
import { Button } from "./Button";
import { useThemeTokens } from "../theme/theme";
export function AsyncState({
  loading,
  error,
  empty,
  onRetry,
}: {
  loading?: boolean;
  error?: string | null;
  empty?: string;
  onRetry?: () => void;
}) {
  const t = useThemeTokens();
  return (
    <View accessibilityLiveRegion="polite" className="min-w-0 gap-4 py-6">
      {loading ? (
        <ActivityIndicator
          accessibilityLabel="Chargement"
          color={t.colors.textPrimary}
        />
      ) : null}
      <Text
        style={{ color: error ? t.colors.dangerText : t.colors.textPrimary }}
      >
        {error ?? (loading ? "Chargement…" : empty)}
      </Text>
      {error && onRetry ? <Button onPress={onRetry}>Réessayer</Button> : null}
    </View>
  );
}
