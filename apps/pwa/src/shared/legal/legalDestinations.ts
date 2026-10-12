import { readPublicEnv } from "@shared/config/runtimeEnv";
import { legalLinksForOrigin } from "./legalOrigin";

export function getLegalLinks() {
  return legalLinksForOrigin(readPublicEnv("EXPO_PUBLIC_LANDING_ORIGIN"));
}
