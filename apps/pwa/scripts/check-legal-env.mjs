import { load } from "@expo/env";
import { parseLegalOrigin } from "../src/shared/legal/legalOrigin.ts";

// Charger les mêmes .env qu'Expo avant l'export statique, puis refuser une origine absente.
process.env.NODE_ENV ??= "production";
load(process.cwd());
parseLegalOrigin(process.env.EXPO_PUBLIC_LANDING_ORIGIN);
console.log("[legal-env] Origine des documents publics valide.");
