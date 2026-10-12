import {
  expoClient,
  getSetCookie,
  hasBetterAuthCookies,
  storageAdapter,
} from "@better-auth/expo/client";
import { jwtClient } from "better-auth/client/plugins";
import { createAuthClient } from "better-auth/react";
import * as SecureStore from "expo-secure-store";
import { Platform } from "react-native";

import { readPublicEnv } from "@shared/config/runtimeEnv";

export const betterAuthBaseUrl = readPublicEnv("EXPO_PUBLIC_BETTER_AUTH_URL")
  .replace(/\/$/, "");

export const isBetterAuthConfigured = Boolean(betterAuthBaseUrl);

const authStorage = storageAdapter(SecureStore);
const authCookieStorageKey = "filfinance-auth_cookie";

export const betterAuthClient = createAuthClient({
  baseURL: betterAuthBaseUrl || "http://localhost:3000",
  plugins: [
    expoClient({
      scheme: "filfinance",
      storagePrefix: "filfinance-auth",
      storage: SecureStore,
    }),
    jwtClient(),
  ],
});

export async function consumeExpoAuthRedirectCookie(setCookieHeader: string): Promise<boolean> {
  if (Platform.OS === "web" || !hasBetterAuthCookies(setCookieHeader, "better-auth")) {
    return false;
  }

  const previousCookie = authStorage.getItem(authCookieStorageKey) ?? undefined;
  await authStorage.setItem(
    authCookieStorageKey,
    getSetCookie(setCookieHeader, previousCookie),
  );
  return true;
}
