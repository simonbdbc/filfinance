import type { AuthSessionClient } from "@shared/api/authSession";
import { Text } from "@shared/ui/Text";
import { View } from "react-native";
export type CalendarLogsPanelProps = {
  authSession: AuthSessionClient | null;
  wsUrl: string;
};
/** The reference file reader targets the PWA (DOM text selection and byte window). */
export function CalendarLogsPanel(_props: CalendarLogsPanelProps) {
  return (
    <View>
      <Text>Le lecteur de fichiers est disponible dans la PWA Fil Finance.</Text>
    </View>
  );
}
