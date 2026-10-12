import { Button } from "@shared/ui/Button";
import { Icon } from "@shared/ui/Icon";
import { BrandMark } from "@shared/ui/BrandMark";
import type { PropsWithChildren } from "react";
import { View } from "react-native";

import { useTheme } from "@shared/theme/theme";
import { PageViewport } from "@shared/ui/PageViewport";
import { LegalLinks } from "@shared/legal/LegalLinks";

export function AuthBrandHeader() {
  return (
    <View className="w-full max-w-full min-w-0 items-center pb-2 pt-4">
      <BrandMark variant="auth" />
    </View>
  );
}

export function AuthPageShell({ children }: PropsWithChildren) {
  const { themeName, setThemeName } = useTheme();
  return (
    <PageViewport>
      <View className="mx-auto w-full min-w-0 gap-4" style={{ maxWidth: 560 }}>
        <View className="items-end">
          <Button
            accessibilityLabel="Basculer entre les thèmes clair et sombre"
            onPress={() =>
              setThemeName(themeName === "light" ? "dark" : "light")
            }
            prefix={<Icon name={themeName === "light" ? "moon" : "sun"} />}
          />
        </View>
        {children}
        <View style={{ paddingTop: 24 }}>
          <LegalLinks />
        </View>
      </View>
    </PageViewport>
  );
}
