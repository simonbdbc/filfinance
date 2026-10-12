import { ActivityIndicator } from "react-native";

import { useThemeTokens } from "../theme/theme";
import { Surface, SectionHeader } from "./Surface";
import { SubtleText } from "./Typography";

export function UserProfileLoadingCard({ error }: { error?: string | null }) {
  const tokens = useThemeTokens();

  if (error) {
    return (
      <Surface variant="danger">
        <SectionHeader title="Profil utilisateur indisponible" />
        <SubtleText style={{ color: tokens.colors.dangerText }}>
          {error}
        </SubtleText>
      </Surface>
    );
  }

  return (
    <Surface>
      <SectionHeader
        title="Chargement du profil utilisateur"
        description="Chargement du rôle et des préférences du compte."
        action={<ActivityIndicator color={tokens.colors.textPrimary} />}
      />
    </Surface>
  );
}
