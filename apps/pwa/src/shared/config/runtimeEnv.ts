type PublicEnvKey =
  | "EXPO_PUBLIC_LANDING_ORIGIN"
  | "EXPO_PUBLIC_FILFINANCE_WS_URL"
  | "EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL"
  | "EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL"
  | "EXPO_PUBLIC_FILFINANCE_PWA_URL"
  | "EXPO_PUBLIC_API_URL"
  | "EXPO_PUBLIC_BETTER_AUTH_URL";

type PublicRuntimeEnv = Partial<Record<PublicEnvKey, string>>;

declare global {
  interface Window {
    __FILFINANCE_ENV__?: PublicRuntimeEnv;
  }
}

const buildTimeEnv: PublicRuntimeEnv = {
  EXPO_PUBLIC_LANDING_ORIGIN: process.env.EXPO_PUBLIC_LANDING_ORIGIN,
  EXPO_PUBLIC_FILFINANCE_WS_URL: process.env.EXPO_PUBLIC_FILFINANCE_WS_URL,
  EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL:
    process.env.EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL,
  EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL:
    process.env.EXPO_PUBLIC_FILFINANCE_HLS_BASE_URL,
  EXPO_PUBLIC_FILFINANCE_PWA_URL:
    process.env.EXPO_PUBLIC_FILFINANCE_PWA_URL,
  EXPO_PUBLIC_API_URL: process.env.EXPO_PUBLIC_API_URL,
  EXPO_PUBLIC_BETTER_AUTH_URL: process.env.EXPO_PUBLIC_BETTER_AUTH_URL,
};

function cleanEnvValue(value: string | undefined) {
  const cleaned = value?.trim();
  return cleaned ? cleaned : undefined;
}

export function readPublicEnv(key: PublicEnvKey) {
  if (typeof window !== "undefined") {
    const runtimeValue = cleanEnvValue(window.__FILFINANCE_ENV__?.[key]);
    if (runtimeValue) {
      return runtimeValue;
    }
  }

  return cleanEnvValue(buildTimeEnv[key]) ?? "";
}
