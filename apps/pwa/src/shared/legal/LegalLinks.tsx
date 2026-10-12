import { Text } from "@shared/ui/Text";
import { useState, type PropsWithChildren } from "react";
import { Linking, Platform, Pressable, View } from "react-native";
import { useThemeTokens } from "@shared/theme/theme";
import { getLegalLinks } from "./legalDestinations";

function LegalLink({ href, children }: PropsWithChildren<{ href: string }>) {
  const tokens = useThemeTokens();
  const [error, setError] = useState("");
  // Un vrai lien Web conserve le formulaire ouvert, sans recopier de données dans l'URL.
  if (Platform.OS === "web") {
    return (
      <a
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        aria-label={`${children} (nouvel onglet)`}
        style={{
          color: tokens.colors.textPrimary,
          fontFamily: "inherit",
          fontSize: 14,
          lineHeight: "22px",
          display: "inline-flex",
          alignItems: "center",
          minHeight: 44,
          textDecoration: "underline",
          padding: "0 4px",
        }}
      >
        {children}
      </a>
    );
  }
  return (
    <View>
      <Pressable
        accessibilityRole="link"
        accessibilityHint="Ouvre le navigateur"
        style={{
          minHeight: 44,
          justifyContent: "center",
          paddingHorizontal: 4,
        }}
        onPress={() => {
          setError("");
          void Linking.openURL(href).catch(() =>
            setError(
              "Impossible d’ouvrir ce document. Vérifiez votre connexion, puis réessayez.",
            ),
          );
        }}
      >
        <Text
          style={{
            color: tokens.colors.textPrimary,
            textDecorationLine: "underline",
            fontSize: 14,
          }}
        >
          {children}
        </Text>
      </Pressable>
      {error ? (
        <Text
          accessibilityLiveRegion="polite"
          style={{ color: tokens.colors.textPrimary }}
        >
          {error}
        </Text>
      ) : null}
    </View>
  );
}

export function LegalLinks({
  registration = false,
  vertical = false,
}: {
  registration?: boolean;
  vertical?: boolean;
}) {
  const tokens = useThemeTokens();
  let links;
  try {
    links = getLegalLinks();
  } catch {
    return (
      <Text
        accessibilityRole="alert"
        style={{ color: tokens.colors.textPrimary }}
      >
        Documents momentanément indisponibles. Contact : support@filfinance.fr.
      </Text>
    );
  }
  return (
    <View style={{ gap: 8, maxWidth: "100%", minWidth: 0 }}>
      {registration ? (
        <>
          <Text
            style={{
              color: tokens.colors.textPrimary,
              fontSize: 14,
              lineHeight: 21,
            }}
          >
            SB ALGO utilise votre adresse e-mail et les informations de votre compte pour
            vous donner accès à Fil Finance et sécuriser le service. Pour
            exercer vos droits : support@filfinance.fr. Consultez la politique
            de confidentialité avant de créer votre compte.
          </Text>
          <LegalLink href={links.find((link) => link.key === "privacy")!.href}>
            Politique de confidentialité
          </LegalLink>
          <Text
            style={{
              color: tokens.colors.textPrimary,
              fontSize: 14,
              lineHeight: 21,
            }}
          >
            Les conditions d’utilisation décrivent le service et ses règles
            d’usage.
          </Text>
          <LegalLink href={links.find((link) => link.key === "terms")!.href}>
            Conditions générales d’utilisation
          </LegalLink>
        </>
      ) : (
        <>
          <Text
            accessibilityRole="header"
            style={{
              color: tokens.colors.textPrimary,
              fontWeight: "600",
              fontSize: 16,
            }}
          >
            Informations légales
          </Text>
          <View
            style={{
              flexDirection: vertical ? "column" : "row",
              flexWrap: "wrap",
              gap: 8,
            }}
          >
            {links.map((link) => (
              <LegalLink key={link.key} href={link.href}>
                {link.label}
              </LegalLink>
            ))}
          </View>
        </>
      )}
    </View>
  );
}
