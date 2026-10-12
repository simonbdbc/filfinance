import { appRoleDisplayLabel } from "../auth/roles";
import { useThemeTokens } from "../theme/theme";
import { Surface, SectionHeader } from "./Surface";
import { SubtleText } from "./Typography";

export function AccessDeniedCard({ role }: { role: string | null }) {
  const tokens = useThemeTokens();

  return (
    <Surface variant="danger">
      <SectionHeader title="Accès non autorisé" />
      <SubtleText style={{ color: tokens.colors.dangerText }}>
        Rôle utilisateur courant :{" "}
        {appRoleDisplayLabel(role) ?? "profil en cours de chargement"}.
      </SubtleText>
    </Surface>
  );
}
