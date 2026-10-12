import type { ReactNode } from "react";
import { View } from "react-native";
import { Text } from "./Text";
import { DataRow } from "./Surface";
export function PreferenceRow({
  label,
  children,
  description,
}: {
  label: string;
  children: ReactNode;
  description?: string;
}) {
  return (
    <DataRow>
      <View className="flex-row flex-wrap justify-between items-center gap-4">
        <Text>{label}</Text>
        <View className="min-w-0 max-w-full">{children}</View>
      </View>
      {description ? (
        <Text style={{ fontSize: 14, lineHeight: 21 }}>{description}</Text>
      ) : null}
    </DataRow>
  );
}
