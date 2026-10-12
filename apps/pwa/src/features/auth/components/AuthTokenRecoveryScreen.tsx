import { View } from "react-native";

import { AuthBrandHeader, AuthPageShell } from "./AuthPageShell";
import { useAuth } from "../providers/AuthProvider";
import { Button } from "@shared/ui/Button";
import { Surface, SectionHeader } from "@shared/ui/Surface";

export function AuthTokenRecoveryScreen() {
  const {
    error,
    isLoading,
    isSubmitting,
    clearError,
    refreshSession,
    signOut,
  } = useAuth();

  return (
    <AuthPageShell>
      <View className="w-full max-w-full min-w-0 gap-3">
        <AuthBrandHeader />
        <Surface variant="danger">
          <SectionHeader
            title="Session à restaurer"
            description={
              error ?? "Le jeton API de la session n’est pas encore disponible."
            }
          />
          <View className="w-full max-w-full min-w-0 gap-3 pt-3">
            <Button
              disabled={isLoading || isSubmitting}
              onPress={() => {
                clearError();
                void refreshSession();
              }}
              variant="primary"
            >
              {isLoading ? "Nouvelle tentative…" : "Réessayer"}
            </Button>
            <Button
              disabled={isSubmitting}
              onPress={() => {
                void signOut();
              }}
            >
              {isSubmitting ? "Déconnexion…" : "Se déconnecter"}
            </Button>
          </View>
        </Surface>
      </View>
    </AuthPageShell>
  );
}
