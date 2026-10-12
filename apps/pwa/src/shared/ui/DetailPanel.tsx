import { useRef, type PropsWithChildren, type ReactNode } from "react";
import { ScrollView, View, useWindowDimensions } from "react-native";
import { UIDialog } from "./primitives";
import { Text, headingStyle } from "./Text";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { useThemeTokens } from "../theme/theme";

export function DialogPanel({
  children,
  title,
  isOpen,
  onClose,
}: PropsWithChildren<{ title: string; isOpen: boolean; onClose: () => void }>) {
  const t = useThemeTokens();
  const closeRef = useRef(null);
  return (
    <UIDialog
      isOpen={isOpen}
      onClose={onClose}
      initialFocusRef={closeRef}
      isKeyboardDismissable
      closeOnOverlayClick
      style={{
        position: "absolute",
        inset: 0,
        alignItems: "center",
        justifyContent: "center",
        padding: 16,
      }}
    >
      <UIDialog.Backdrop
        style={{
          position: "absolute",
          inset: 0,
          backgroundColor: t.colors.overlay,
        }}
      />
      <UIDialog.Content
        role="dialog"
        accessibilityLabel={title}
        className="w-full max-w-2xl"
        style={{
          maxHeight: "90%",
          minWidth: 0,
          padding: 24,
          backgroundColor: t.colors.surface,
          borderRadius: t.radii.panel,
        }}
      >
        <UIDialog.Header className="flex-row items-start gap-4 mb-4">
          <Text
            accessibilityRole="header"
            style={[headingStyle(t), { flex: 1 }]}
          >
            {title}
          </Text>
          <UIDialog.CloseButton
            ref={closeRef}
            accessibilityLabel="Fermer le panneau"
            style={{
              minHeight: 44,
              minWidth: 44,
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <Icon name="x" />
          </UIDialog.CloseButton>
        </UIDialog.Header>
        <UIDialog.Body>{children}</UIDialog.Body>
      </UIDialog.Content>
    </UIDialog>
  );
}
export function MasterDetail({
  list,
  children,
  title,
  isOpen,
  onClose,
}: PropsWithChildren<{
  list: ReactNode;
  title: string;
  isOpen: boolean;
  onClose: () => void;
}>) {
  const { width } = useWindowDimensions();
  const t = useThemeTokens();
  return (
    <View
      className="w-full min-w-0"
      style={{ flexDirection: width >= 1280 ? "row" : "column", gap: 24 }}
    >
      <View style={{ flex: 1, minWidth: 0 }}>{list}</View>
      {width >= 1280 ? (
        <View
          style={{
            flex: 1,
            minWidth: 0,
            borderLeftWidth: 1,
            borderColor: t.colors.borderSoft,
            paddingLeft: 24,
          }}
        >
          <Text
            accessibilityRole="header"
            style={[headingStyle(t), { marginBottom: 24 }]}
          >
            {title}
          </Text>
          {isOpen ? (
            children
          ) : (
            <Text style={{ fontSize: 14 }}>
              Sélectionnez une ligne pour consulter son détail.
            </Text>
          )}
        </View>
      ) : (
        <DialogPanel title={title} isOpen={isOpen} onClose={onClose}>
          {children}
        </DialogPanel>
      )}
    </View>
  );
}
