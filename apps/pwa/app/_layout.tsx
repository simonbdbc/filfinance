import "../global.css";

import { Stack } from "expo-router";

import { AuthProvider } from "@features/auth";
import { ThemeProvider } from "@shared/theme/theme";

// La langue du document guide la prononciation des lecteurs d’écran Web.
if (typeof document !== "undefined") document.documentElement.lang = "fr";

export default function RootLayout() {
  return (
    <ThemeProvider>
      <AuthProvider>
        <Stack screenOptions={{ headerShown: false }} />
      </AuthProvider>
    </ThemeProvider>
  );
}
