import { useThemeTokens } from "@shared/theme/theme";
import { Surface } from "@shared/ui/Surface";
import { EyebrowText, SubtleText } from "@shared/ui/Typography";

type ErrorSummaryCardProps = {
  fluxError: string | null;
  audioError: string | null;
  historyError: string | null;
  traceError: string | null;
  profileError: string | null;
};

export function ErrorSummaryCard({
  fluxError,
  audioError,
  historyError,
  traceError,
  profileError,
}: ErrorSummaryCardProps) {
  const tokens = useThemeTokens();
  const errorTextStyle = { color: tokens.colors.dangerText };

  if (
    !fluxError &&
    !audioError &&
    !historyError &&
    !traceError &&
    !profileError
  ) {
    return null;
  }

  return (
    <Surface variant="danger">
      <EyebrowText className="font-extrabold" style={errorTextStyle}>
        Dernières erreurs
      </EyebrowText>
      {fluxError ? (
        <SubtleText className="text-sm leading-5" style={errorTextStyle}>
          Flux : {fluxError}
        </SubtleText>
      ) : null}
      {audioError ? (
        <SubtleText className="text-sm leading-5" style={errorTextStyle}>
          Flux audio : {audioError}
        </SubtleText>
      ) : null}
      {historyError ? (
        <SubtleText className="text-sm leading-5" style={errorTextStyle}>
          Historique : {historyError}
        </SubtleText>
      ) : null}
      {traceError ? (
        <SubtleText className="text-sm leading-5" style={errorTextStyle}>
          Test : {traceError}
        </SubtleText>
      ) : null}
      {profileError ? (
        <SubtleText className="text-sm leading-5" style={errorTextStyle}>
          Profil : {profileError}
        </SubtleText>
      ) : null}
    </Surface>
  );
}
