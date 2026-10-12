import type { PropsWithChildren, ReactNode } from "react";
import { Platform, SafeAreaView, ScrollView, View } from "react-native";
import type { ViewStyle } from "react-native";
import { StatusBar } from "expo-status-bar";

import { useThemeTokens } from "../theme/theme";

const webViewportStyle = {
  height: "100vh",
  width: "100vw",
  maxWidth: "100vw",
  overflowX: "hidden",
} as unknown as ViewStyle;

const webContentStyle = {
  boxSizing: "border-box",
  flexGrow: 1,
  maxWidth: "100vw",
  overflowX: "hidden",
  width: "100%",
} as unknown as ViewStyle;

type PageViewportProps = PropsWithChildren<{
  fixedContent?: ReactNode;
}>;

export function PageViewport({ children, fixedContent }: PageViewportProps) {
  const tokens = useThemeTokens();
  const viewportStyle = Platform.OS === "web" ? webViewportStyle : undefined;
  const contentStyle = Platform.OS === "web" ? webContentStyle : undefined;

  return (
    <SafeAreaView
      className="flex-1"
      style={[viewportStyle, { backgroundColor: tokens.colors.background }]}
    >
      <StatusBar style={tokens.name === "dark" ? "light" : "dark"} />
      <ScrollView
        className="box-border flex-1"
        contentContainerStyle={contentStyle}
        style={[viewportStyle, { backgroundColor: tokens.colors.background }]}
      >
        <View
          className="box-border w-full max-w-full min-w-0 gap-4 p-5"
          style={contentStyle}
        >
          {children}
        </View>
      </ScrollView>
      {fixedContent}
    </SafeAreaView>
  );
}
