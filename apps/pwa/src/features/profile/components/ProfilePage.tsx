import { View, useWindowDimensions } from "react-native";
import { useAuth } from "@features/auth";
import type { CurrentUserProfileResponse } from "@shared/api/contracts";
import { appRoleDisplayLabel } from "@shared/auth/roles";
import { formatLocalDateTime } from "@shared/lib/dateTime";
import { useTheme } from "@shared/theme/theme";
import { Surface, SectionHeader } from "@shared/ui/Surface";
import { Text } from "@shared/ui/Text";
import { Button } from "@shared/ui/Button";
import { Icon } from "@shared/ui/Icon";
import { PreferenceRow } from "@shared/ui/PreferenceGroup";
import { AsyncState } from "@shared/ui/AsyncState";
import { LegalLinks } from "@shared/legal/LegalLinks";

type ProfilePageProps = {
  profile: CurrentUserProfileResponse | null;
  isProfileCurrent: boolean;
  profileError: string | null;
};
export function ProfilePage({
  profile,
  isProfileCurrent,
  profileError,
}: ProfilePageProps) {
  const { themeName, setThemeName } = useTheme();
  const { user, isSubmitting, signOut } = useAuth();
  const { width } = useWindowDimensions();
  return (
    <View style={{ flexDirection: width >= 1280 ? "row" : "column", gap: 24 }}>
      <View style={{ flex: 2, minWidth: 0, gap: 24 }}>
        <Surface>
          <SectionHeader title="Mon compte" hasSeparator={false} />
          {profileError ? (
            <AsyncState error={profileError} />
          ) : !isProfileCurrent ? (
            <AsyncState loading />
          ) : null}
          <PreferenceRow label="Adresse e-mail">
            <Text>{profile?.email ?? user?.email ?? "Non renseignée"}</Text>
          </PreferenceRow>
          <PreferenceRow label="Compte">
            <Text>{appRoleDisplayLabel(profile?.role) ?? "Chargement"}</Text>
          </PreferenceRow>
          {profile ? (
            <>
              <PreferenceRow label="Création">
                <Text>
                  {formatLocalDateTime(profile.created_at, "Non disponible")}
                </Text>
              </PreferenceRow>
              <PreferenceRow label="Mise à jour">
                <Text>
                  {formatLocalDateTime(profile.updated_at, "Non disponible")}
                </Text>
              </PreferenceRow>
            </>
          ) : null}
        </Surface>
        <Surface>
          <SectionHeader
            title="Mes préférences"
            description="Votre espace, selon vos habitudes."
            hasSeparator={false}
          />
          <PreferenceRow
            label="Thème"
            description="Enregistré sur cet appareil, indépendamment du compte et du site public."
          >
            <View className="flex-row flex-wrap gap-2">
              <Button
                selected={themeName === "light"}
                prefix={<Icon name="sun" />}
                onPress={() => setThemeName("light")}
              >
                Clair
              </Button>
              <Button
                selected={themeName === "dark"}
                prefix={<Icon name="moon" />}
                onPress={() => setThemeName("dark")}
              >
                Sombre
              </Button>
            </View>
          </PreferenceRow>
          <PreferenceRow label="Voix TTS préférée">
            <Text>
              {profile?.preferences.preferred_tts_voice ?? "Valeur par défaut du serveur"}
            </Text>
          </PreferenceRow>
        </Surface>
        <Button
          prefix={<Icon name="log-out" />}
          disabled={isSubmitting}
          onPress={() => void signOut()}
        >
          {isSubmitting ? "Déconnexion…" : "Se déconnecter"}
        </Button>
      </View>
      <View style={{ flex: 1, minWidth: 0 }}>
        <Surface>
          <LegalLinks vertical />
        </Surface>
      </View>
    </View>
  );
}
