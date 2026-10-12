import {
  formatLocalDayKey,
  currentParisLocalDayKey,
} from "@shared/lib/dateTime";
import { useEffect, useRef, useState, type PropsWithChildren } from "react";
import {
  Platform,
  ScrollView,
  View,
  useWindowDimensions,
  type ViewStyle,
} from "react-native";
import type { AppRole, AppView } from "@shared/auth/roles";
import { useTheme, type ThemeName } from "@shared/theme/theme";
import { Text } from "@shared/ui/Text";
import { Button } from "@shared/ui/Button";
import { Icon } from "@shared/ui/Icon";
import { DialogPanel } from "@shared/ui/DetailPanel";
import { AudioPlayerView, type AudioControl } from "@shared/ui/AudioPlayerView";
import { appViewDisplayNameByView } from "../navigation/routes";
import { PageHeader, PageHeaderInLayout } from "@shared/ui/PageHeader";
import { AppNavigation } from "../components/AppNavigation";

import { BrandMark } from "@shared/ui/BrandMark";
export function AppLayout({
  activeView,
  pageKey,
  audioControl,
  role,
  onThemeNameChange,
  onViewChange,
  children,
}: PropsWithChildren<{
  activeView: AppView;
  pageKey: string;
  audioControl: AudioControl;
  role: AppRole | null;
  onThemeNameChange: (themeName: ThemeName) => void;
  onViewChange: (view: AppView) => void;
}>) {
  const { tokens: t, themeName } = useTheme();
  const { width } = useWindowDimensions();
  const desktop = width >= 1024;
  const [menuOpen, setMenuOpen] = useState(false);
  const contentRef = useRef<ScrollView>(null);
  // La navigation remet la page en haut sans remonter le lecteur audio.
  useEffect(() => {
    contentRef.current?.scrollTo({ y: 0, animated: false });
  }, [pageKey]);
  const navigate = (view: AppView) => {
    setMenuOpen(false);
    onViewChange(view);
  };
  const brand = <BrandMark variant={desktop ? "desktop" : "mobile"} />;
  const themeButton = (
    <Button
      accessibilityLabel="Basculer entre les thèmes clair et sombre"
      onPress={() =>
        onThemeNameChange(themeName === "light" ? "dark" : "light")
      }
      prefix={<Icon name={themeName === "light" ? "moon" : "sun"} />}
    >
      {themeName === "light" ? "Thème sombre" : "Thème clair"}
    </Button>
  );
  const navigation = (
    <AppNavigation
      activeView={activeView}
      role={role}
      onViewChange={navigate}
    />
  );
  return (
    <View
      style={{
        flex: 1,
        minHeight: 0,
        minWidth: 0,
        backgroundColor: t.colors.background,
      }}
    >
      {desktop ? (
        <View
          style={
            {
              position: Platform.OS === "web" ? "fixed" : "absolute",
              top: 0,
              bottom: 0,
              left: 0,
              width: 248,
              maxHeight: "100vh",
              backgroundColor: t.colors.surface,
              borderRightWidth: 1,
              borderColor: t.colors.borderSoft,
              padding: 24,
              gap: 32,
            } as unknown as ViewStyle
          }
        >
          {brand}
          <ScrollView
            style={{ flex: 1 }}
            contentContainerStyle={{ paddingBottom: 24 }}
          >
            {navigation}
          </ScrollView>
          {themeButton}
        </View>
      ) : (
        <View className="flex-row items-center justify-between gap-2 px-4 pt-4">
          <Button
            accessibilityLabel="Ouvrir le menu"
            onPress={() => setMenuOpen(true)}
            prefix={<Icon name="menu" />}
          />
          {brand}
          <Button
            accessibilityLabel="Basculer entre les thèmes clair et sombre"
            onPress={() =>
              onThemeNameChange(themeName === "light" ? "dark" : "light")
            }
            prefix={<Icon name={themeName === "light" ? "moon" : "sun"} />}
          />
        </View>
      )}
      <ScrollView
        ref={contentRef}
        testID="page-scroll"
        tabIndex={0}
        accessibilityLabel="Contenu de la page"
        style={{
          marginLeft: desktop ? 248 : 0,
          minHeight: 0,
          minWidth: 0,
          flex: 1,
        }}
        contentContainerStyle={{
          padding: desktop ? 32 : 16,
          paddingBottom: desktop ? 32 : 96,
          minWidth: 0,
          flexGrow: 1,
        }}
      >
        <View
          className="mx-auto w-full min-w-0 gap-6"
          style={{ maxWidth: 1500 }}
        >
          <PageHeader
            title={appViewDisplayNameByView[activeView]}
            action={
              activeView === "feed" ? (
                <Text style={{ fontSize: 14 }}>
                  {formatLocalDayKey(currentParisLocalDayKey())}
                </Text>
              ) : undefined
            }
          />
          <AudioPlayerView control={audioControl} />
          <PageHeaderInLayout.Provider value={true}>
            {children}
          </PageHeaderInLayout.Provider>
        </View>
      </ScrollView>
      {!desktop ? (
        <View
          style={
            {
              position: Platform.OS === "web" ? "fixed" : "absolute",
              bottom: 0,
              left: 0,
              right: 0,
              zIndex: 20,
              backgroundColor: t.colors.surface,
              borderTopWidth: 1,
              borderColor: t.colors.borderSoft,
            } as unknown as ViewStyle
          }
        >
          <AppNavigation
            activeView={activeView}
            role={role}
            onViewChange={navigate}
            compact
          />
        </View>
      ) : null}
      <DialogPanel
        title="Navigation"
        isOpen={menuOpen && !desktop}
        onClose={() => setMenuOpen(false)}
      >
        {navigation}
        <View className="mt-6">{themeButton}</View>
      </DialogPanel>
    </View>
  );
}
