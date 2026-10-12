import { TextInput } from "@shared/ui/TextInput";
import { Text } from "@shared/ui/Text";
import { useEffect, useRef, useState } from "react";
import { View, useWindowDimensions } from "react-native";
import type {
  NativeScrollEvent,
  NativeSyntheticEvent,
  ScrollView as ScrollViewType,
} from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type { AdminLogLine, ConnectionState } from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import { connectionLabel } from "@shared/lib/status";
import { useThemeTokens } from "@shared/theme/theme";
import {
  AccessDeniedCard,
  Button,
  SubtleText,
  UserProfileLoadingCard,
  Select,
} from "@shared/ui";
import {
  FilterRail,
  Grid,
  InspectorPanel,
  KpiStrip,
  LogToolbar,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
  TerminalPanel,
  TriPane,
  terminalTextStyle,
} from "@shared/ui/Workspace";
import { useAdminLogs } from "../hooks/useAdminLogs";
import { useAdminLogTail } from "../hooks/useAdminLogTail";
import {
  adminLogLevelOptions,
  useAdminLogsViewModel,
} from "../hooks/useAdminLogsViewModel";
import {
  connectionTone,
  formatBytes,
  formatTerminalLine,
  terminalColorForLevel,
} from "../services/dashboardFormatters";

export function AdminLogsScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const { width: viewportWidth } = useWindowDimensions();
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();
  const isWideLayout = viewportWidth >= 1080;
  const logs = useAdminLogs({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });
  const liveLogs = useAdminLogTail({
    authSession,
    day: logs.selectedDay,
    isEnabled: canAccessAdmin,
    level: logs.level,
    query: logs.query,
    wsUrl: server.wsUrl,
  });
  const viewModel = useAdminLogsViewModel({
    connectionState: liveLogs.connectionState,
    days: logs.days,
    items: liveLogs.items,
    lastFileSizeBytes: liveLogs.lastFileSizeBytes,
    lastHeartbeat: liveLogs.lastHeartbeat,
    level: logs.level,
    query: logs.query,
    selectedDay: logs.selectedDay,
  });

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminLogs}
      description="Lecture bornée des journaux du serveur conservés pour aujourd’hui et hier."
      commandBar={
        <LogToolbar
          actions={[
            {
              disabled: logs.isLoading,
              label: "Actualiser",
              onPress: logs.reload,
            },
            {
              label: "Effacer la vue",
              onPress: liveLogs.clear,
            },
          ]}
          status={
            <>
              <StateIndicator tone={connectionTone(liveLogs.connectionState)}>
                {connectionLabel(liveLogs.connectionState)}
              </StateIndicator>
              <StateIndicator tone="muted">
                Lignes : {liveLogs.items.length}
              </StateIndicator>
            </>
          }
        />
      }
    >
      <KpiStrip items={viewModel.metrics} />

      <TriPane
        isWide={isWideLayout}
        railWidth={320}
        inspectorWidth={320}
        rail={
          <Grid direction="vertical">
            <FilterRail
              title="Filtres"
              description="Filtres de jour, de niveau et de recherche appliqués au suivi en direct."
            >
              <View className="w-full max-w-full min-w-0 gap-2">
                <SubtleText>Jour</SubtleText>
                <Select
                  label="Jour des journaux"
                  disabled={logs.isLoading || logs.days.length === 0}
                  value={logs.selectedDay ?? ""}
                  options={viewModel.dayOptions}
                  onChange={(day) => {
                    logs.setSelectedDay(day || null);
                  }}
                />
              </View>
              <View className="w-full max-w-full min-w-0 gap-2">
                <SubtleText>Niveau</SubtleText>
                <Select
                  label="Niveau des journaux"
                  disabled={logs.isLoading}
                  value={logs.level ?? ""}
                  options={adminLogLevelOptions}
                  onChange={(level) => {
                    logs.setLevel(level || null);
                  }}
                />
              </View>
              <View className="w-full max-w-full min-w-0 gap-2">
                <SubtleText>Recherche</SubtleText>
                <TextInput
                  className="box-border w-full max-w-full min-w-0 border px-3 py-3 text-sm"
                  onChangeText={logs.setQuery}
                  placeholder="Texte à rechercher"
                  placeholderTextColor={tokens.colors.textPrimary}
                  style={{
                    backgroundColor: dashboardTheme.colors.surface,
                    borderColor: dashboardTheme.colors.border,
                    borderRadius: dashboardTheme.radii.control,
                    color: tokens.colors.textPrimary,
                  }}
                  value={logs.query}
                />
              </View>
              {logs.error ? (
                <SubtleText style={{ color: tokens.colors.dangerText }}>
                  {logs.error}
                </SubtleText>
              ) : null}
            </FilterRail>

            <AdminLogLiveControls
              connectionState={liveLogs.connectionState}
              disabled={!logs.selectedDay}
              error={liveLogs.error}
              isDirectEnabled={liveLogs.isDirectEnabled}
              lastFileSizeBytes={liveLogs.lastFileSizeBytes}
              lastHeartbeat={liveLogs.lastHeartbeat}
              onClear={liveLogs.clear}
              onPause={liveLogs.pauseDirect}
              onReconnect={liveLogs.reconnect}
              onResume={liveLogs.resumeDirect}
              onStart={liveLogs.startDirect}
            />
          </Grid>
        }
        main={<AdminLiveLogTerminal items={liveLogs.items} />}
        inspector={
          <InspectorPanel
            title="Détails de la session"
            description="État de la connexion WebSocket et du fichier sélectionné."
            fields={viewModel.inspectorFields}
          />
        }
      />
    </WorkspacePage>
  );
}

function AdminLogLiveControls({
  connectionState,
  disabled,
  error,
  isDirectEnabled,
  lastFileSizeBytes,
  lastHeartbeat,
  onClear,
  onPause,
  onReconnect,
  onResume,
  onStart,
}: {
  connectionState: ConnectionState;
  disabled: boolean;
  error: string | null;
  isDirectEnabled: boolean;
  lastFileSizeBytes: number | null;
  lastHeartbeat: string | null;
  onClear: () => void;
  onPause: () => void;
  onReconnect: () => void;
  onResume: () => void;
  onStart: () => void;
}) {
  const tokens = useThemeTokens();
  const primaryActionLabel = isDirectEnabled ? "Pause" : "Direct";

  return (
    <SectionPanel
      title="Suivi en direct"
      description="Lecture WebSocket du fichier journal sélectionné."
      action={
        <StateIndicator tone={connectionTone(connectionState)}>
          {connectionLabel(connectionState)}
        </StateIndicator>
      }
    >
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
        <Button
          className="min-w-[110px] flex-1"
          disabled={disabled}
          onPress={isDirectEnabled ? onPause : onStart}
          variant="primary"
        >
          {primaryActionLabel}
        </Button>
        <Button
          className="min-w-[110px] flex-1"
          disabled={disabled || isDirectEnabled}
          onPress={onResume}
        >
          Reprendre
        </Button>
        <Button
          className="min-w-[110px] flex-1"
          disabled={disabled}
          onPress={onReconnect}
        >
          Reconnexion
        </Button>
        <Button className="min-w-[110px] flex-1" onPress={onClear}>
          Effacer la vue
        </Button>
      </View>
      <View className="w-full max-w-full min-w-0 gap-1">
        <SubtleText>
          Taille observée : {formatBytes(lastFileSizeBytes)}
        </SubtleText>
        <SubtleText>
          Signal de présence : {formatLocalDateTimeWithRaw(lastHeartbeat)}
        </SubtleText>
        {error ? (
          <SubtleText style={{ color: tokens.colors.dangerText }}>
            {error}
          </SubtleText>
        ) : null}
      </View>
    </SectionPanel>
  );
}

function AdminLiveLogTerminal({ items }: { items: AdminLogLine[] }) {
  const tokens = useThemeTokens();
  const scrollRef = useRef<ScrollViewType | null>(null);
  const [shouldAutoScroll, setShouldAutoScroll] = useState(true);

  useEffect(() => {
    if (!shouldAutoScroll) {
      return;
    }
    const id = setTimeout(() => {
      scrollRef.current?.scrollToEnd({ animated: false });
    }, 0);

    return () => clearTimeout(id);
  }, [items.length, shouldAutoScroll]);

  return (
    <TerminalPanel
      title="Terminal en direct"
      description={`Lignes conservées dans la vue : ${items.length}`}
      scrollRef={scrollRef}
      onContentSizeChange={() => {
        if (shouldAutoScroll) {
          scrollRef.current?.scrollToEnd({ animated: false });
        }
      }}
      onScroll={(event) => {
        setShouldAutoScroll(isTerminalScrolledToBottom(event));
      }}
      scrollEventThrottle={80}
    >
      {items.length === 0 ? (
        <Text
          className="text-sm"
          style={[terminalTextStyle, { color: tokens.colors.textPrimary }]}
        >
          Activez le mode Direct pour suivre les journaux en temps réel.
        </Text>
      ) : (
        items.map((item) => (
          <Text
            key={`${item.cursor}-${item.text}`}
            className="text-sm leading-[18px]"
            style={[
              terminalTextStyle,
              { color: terminalColorForLevel(item.level, tokens.colors) },
            ]}
          >
            {formatTerminalLine(item)}
          </Text>
        ))
      )}
    </TerminalPanel>
  );
}

function isTerminalScrolledToBottom(
  event: NativeSyntheticEvent<NativeScrollEvent>,
) {
  const { contentOffset, contentSize, layoutMeasurement } = event.nativeEvent;
  return contentSize.height - contentOffset.y - layoutMeasurement.height < 28;
}
