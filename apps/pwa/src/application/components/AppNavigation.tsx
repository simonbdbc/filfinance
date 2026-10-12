import { View } from "react-native";
import type { AppRole, AppView } from "@shared/auth/roles";
import { canAccessAdmin } from "@shared/auth/roles";
import { useThemeTokens } from "@shared/theme";
import { Text } from "@shared/ui/Text";
import { Icon, type IconName } from "@shared/ui/Icon";
import { UIPressable } from "@shared/ui/primitives";
import { appViewDisplayNameByView } from "../navigation/routes";

const mainItems: { view: AppView; label: string; icon: IconName }[] = [
  { view: "feed", label: "Actualités", icon: "file-text" },
  { view: "calendar", label: "Calendrier", icon: "calendar" },
  { view: "profile", label: "Profil", icon: "user" },
];
const adminViews: AppView[] = [
  "admin",
  "adminConfigSources",
  "adminFluxStatus",
  "adminBlacklist",
  "adminGlossary",
  "adminHistory",
  "adminCalendarHistory",
  "adminDailySummary",
  "adminLogs",
];
export function AppNavigation({
  activeView,
  role,
  onViewChange,
  compact = false,
}: {
  activeView: AppView;
  role: AppRole | null;
  onViewChange: (view: AppView) => void;
  compact?: boolean;
}) {
  const t = useThemeTokens();
  const items = [
    ...mainItems,
    ...(canAccessAdmin(role)
      ? [
          {
            view: "admin" as AppView,
            label: "Administration",
            icon: "settings" as IconName,
          },
        ]
      : []),
  ];
  return (
    <View
      role="navigation"
      accessibilityLabel="Navigation principale"
      style={{
        flexDirection: compact ? "row" : "column",
        gap: compact ? 0 : 8,
      }}
    >
      {items.map((item) => {
        const selected =
          item.view === "admin"
            ? activeView.startsWith("admin")
            : activeView === item.view;
        return (
          <View
            key={item.view}
            style={{ flex: compact ? 1 : undefined, minWidth: 0 }}
          >
            <UIPressable
              accessibilityRole="link"
              aria-current={selected ? "page" : undefined}
              accessibilityLabel={item.label}
              onPress={() => onViewChange(item.view)}
              style={{
                minHeight: compact ? 64 : 52,
                minWidth: 44,
                paddingHorizontal: compact ? 4 : 16,
                paddingVertical: 12,
                flexDirection: compact ? "column" : "row",
                alignItems: "center",
                gap: 8,
                backgroundColor: selected ? t.colors.background : "transparent",
                borderRadius: t.radii.control,
                borderLeftWidth: !compact && selected ? 3 : 0,
                borderColor: t.colors.textPrimary,
              }}
            >
              <Icon name={item.icon} />
              <Text
                style={{
                  fontSize: compact ? 14 : 16,
                  lineHeight: 20,
                  fontWeight: selected ? "600" : "400",
                }}
              >
                {compact && item.view === "admin" ? "Admin" : item.label}
              </Text>
            </UIPressable>
            {!compact && item.view === "admin" && selected ? (
              <View className="ml-4 mt-2 gap-1">
                {adminViews.map((view) => (
                  <UIPressable
                    key={view}
                    accessibilityRole="link"
                    aria-current={activeView === view ? "page" : undefined}
                    onPress={() => onViewChange(view)}
                    style={{
                      minHeight: 44,
                      padding: 12,
                      borderRadius: t.radii.control,
                      backgroundColor:
                        activeView === view
                          ? t.colors.background
                          : "transparent",
                    }}
                  >
                    <Text style={{ fontSize: 14 }}>
                      {appViewDisplayNameByView[view]}
                    </Text>
                  </UIPressable>
                ))}
              </View>
            ) : null}
          </View>
        );
      })}
    </View>
  );
}
