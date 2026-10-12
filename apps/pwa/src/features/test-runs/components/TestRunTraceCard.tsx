import { Text } from "@shared/ui/Text";
import { View } from "react-native";

import { DataRow, SectionPanel } from "@shared/ui/Workspace";

import type { TestTraceEvent } from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import { traceStatusLabel } from "@shared/lib/status";
import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { SubtleText } from "@shared/ui/Typography";

type TestRunTraceCardProps = {
  items: TestTraceEvent[];
  lastTraceItem: TestTraceEvent | null;
  isCollapsed: boolean;
  onCollapsedChange: (isCollapsed: boolean) => void;
};

export function TestRunTraceCard({
  items,
  lastTraceItem,
  isCollapsed,
  onCollapsedChange,
}: TestRunTraceCardProps) {
  const tokens = useThemeTokens();

  return (
    <SectionPanel
      title="Trace du test"
      description={
        lastTraceItem
          ? `${traceStatusLabel(lastTraceItem.status)} • ${lastTraceItem.stage}`
          : "Aucune trace reçue pour le moment."
      }
      action={
        <Button
          className="min-w-[104px]"
          onPress={() => onCollapsedChange(!isCollapsed)}
        >
          {isCollapsed ? "Afficher" : "Réduire"}
        </Button>
      }
    >
      {!isCollapsed ? (
        items.length === 0 ? (
          <SubtleText>Aucune trace reçue pour le moment.</SubtleText>
        ) : (
          items.map((item, index) => (
            <DataRow key={`${item.run_id}-${item.timestamp}-${index}`}>
              <View className="flex-row items-center justify-between gap-3">
                <Text
                  className="text-sm font-extrabold uppercase tracking-[0.8px]"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {item.stage}
                </Text>
                <Text
                  className="text-sm font-extrabold tracking-[0.8px]"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {traceStatusLabel(item.status)}
                </Text>
              </View>
              <Text
                className="text-sm leading-5"
                style={{ color: tokens.colors.textSecondary }}
              >
                {item.message}
              </Text>
              <Text
                className="text-sm"
                style={{ color: tokens.colors.textPrimary }}
              >
                {formatLocalDateTimeWithRaw(item.timestamp)}
              </Text>
              {item.detail ? (
                <Text
                  className="text-sm leading-[18px]"
                  style={{ color: tokens.colors.textPrimary }}
                >
                  {item.detail}
                </Text>
              ) : null}
            </DataRow>
          ))
        )
      ) : null}
    </SectionPanel>
  );
}
