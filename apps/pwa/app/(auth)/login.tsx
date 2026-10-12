import { Redirect } from "expo-router";

import { AuthGate } from "@features/auth";

export default function LoginRoute() {
  return (
    <AuthGate>
      <Redirect href="/feed" />
    </AuthGate>
  );
}
