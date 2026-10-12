import { Text, headingStyle } from "@shared/ui/Text";
import type { PropsWithChildren, ReactNode, RefObject } from "react";
import { Children, isValidElement } from "react";
import {
  ActivityIndicator,
  Platform,
  ScrollView,
  View,
  useWindowDimensions,
} from "react-native";
import type {
  NativeScrollEvent,
  NativeSyntheticEvent,
  ScrollView as ScrollViewType,
  StyleProp,
  TextStyle,
  ViewStyle,
} from "react-native";

import { useThemeTokens } from "@shared/theme/theme";
import { Button } from "@shared/ui/Button";
import { StatusBadge } from "@shared/ui/StatusBadge";
import { Surface, DataRow, SectionHeader } from "@shared/ui/Surface";
import { PageHeader } from "@shared/ui/PageHeader";
import { MetaText, SubtleText } from "@shared/ui/Typography";
import { badgeVariantForTone, toneLabel } from "./statusLabels";
import type {
  CommandAction,
  DataGridColumn,
  GridAreaProps,
  GridProps,
  HealthItem,
  InspectorField,
  Metric,
  TimelineItem,
  Tone,
  TriPaneProps,
  WorkflowStage,
  WorkflowStep,
} from "./workspaceTypes";

type HeaderAction = ReactNode;

export function WorkspacePage({
  actions,
  children,
  commandBar,
  description,
  title,
}: PropsWithChildren<{
  actions?: HeaderAction;
  commandBar?: ReactNode;
  description: string;
  title: string;
}>) {
  return (
    <View className="w-full max-w-full min-w-0 gap-4">
      <PageHeader title={title} description={description} action={actions} />
      {commandBar}
      {children}
    </View>
  );
}

export function CommandBar({
  actions = [],
  children,
  description,
  meta,
  title,
}: PropsWithChildren<{
  actions?: CommandAction[];
  description?: ReactNode;
  meta?: ReactNode;
  title: ReactNode;
}>) {
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-3 border px-4 py-3"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.border,
        borderRadius: dashboardTheme.radii.panel,
      }}
    >
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap items-start justify-between gap-3">
        <View className="min-w-0 flex-1 gap-1">
          <Text
            accessibilityRole="header"
            aria-level={2}
            className="max-w-full min-w-0 break-words"
            style={headingStyle(dashboardTheme)}
          >
            {title}
          </Text>
          {description ? <SubtleText>{description}</SubtleText> : null}
          {meta ? (
            <View className="flex-row flex-wrap gap-2">{meta}</View>
          ) : null}
        </View>
        {actions.length > 0 ? (
          <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
            {actions.map((action) => (
              <Button
                className="min-w-[108px]"
                disabled={action.disabled}
                key={action.label}
                onPress={action.onPress}
                variant={action.variant}
              >
                {action.label}
              </Button>
            ))}
          </View>
        ) : null}
      </View>
      {children}
    </View>
  );
}

export function Section({
  children,
  description,
  title,
}: PropsWithChildren<{
  description?: ReactNode;
  title?: ReactNode;
}>) {
  const dashboardTheme = useThemeTokens();

  return (
    <View className="w-full max-w-full min-w-0 gap-3">
      {title || description ? (
        <View className="gap-1">
          {title ? (
            <Text
              accessibilityRole="header"
              aria-level={2}
              className="max-w-full min-w-0 break-words"
              style={headingStyle(dashboardTheme)}
            >
              {title}
            </Text>
          ) : null}
          {description ? <SubtleText>{description}</SubtleText> : null}
        </View>
      ) : null}
      {children}
    </View>
  );
}

export function SectionPanel({
  action,
  children,
  description,
  title,
}: PropsWithChildren<{
  action?: ReactNode;
  description?: ReactNode;
  title: ReactNode;
}>) {
  return (
    <Surface>
      <SectionHeader title={title} description={description} action={action} />
      {children}
    </Surface>
  );
}

export function KpiStrip({
  items,
  minItemWidth = 210,
}: {
  items: Metric[];
  minItemWidth?: number;
}) {
  return (
    <Grid direction="horizontal" minItemWidth={minItemWidth}>
      {items.map((item) => (
        <MetricTile key={item.label} {...item} />
      ))}
    </Grid>
  );
}

export function Grid({
  children,
  direction = "vertical",
  minItemWidth = 220,
  style,
}: GridProps) {
  const isHorizontal = direction === "horizontal";

  return (
    <View
      className="w-full max-w-full min-w-0 gap-3"
      style={[
        {
          flexDirection: isHorizontal ? "row" : "column",
          flexWrap: isHorizontal ? "wrap" : "nowrap",
        },
        style,
      ]}
    >
      {Children.toArray(children).map((child, index) =>
        isGridAreaElement(child) ? (
          child
        ) : (
          <GridArea
            key={index}
            minWidth={isHorizontal ? minItemWidth : undefined}
          >
            {child}
          </GridArea>
        ),
      )}
    </View>
  );
}

export function GridArea({
  children,
  minWidth,
  span = "auto",
  style,
}: GridAreaProps) {
  const isFull = span === "full";

  return (
    <View
      className="min-w-0"
      style={[
        {
          flexBasis: isFull ? "100%" : minWidth,
          flexGrow: isFull ? 0 : 1,
          flexShrink: 1,
          maxWidth: "100%",
          minWidth: 0,
          width: isFull ? "100%" : undefined,
        },
        style,
      ]}
    >
      {children}
    </View>
  );
}

export function WorkspaceGrid({
  children,
  minItemWidth = 220,
}: PropsWithChildren<{
  minItemWidth?: number;
}>) {
  return (
    <Grid direction="horizontal" minItemWidth={minItemWidth}>
      {children}
    </Grid>
  );
}

function isGridAreaElement(child: ReactNode) {
  return isValidElement(child) && child.type === GridArea;
}

export function StateIndicator({
  children,
  tone = "muted",
}: PropsWithChildren<{
  tone?: Tone;
}>) {
  return (
    <StatusBadge variant={badgeVariantForTone(tone)}>
      {children ?? toneLabel(tone)}
    </StatusBadge>
  );
}

export function MetricTile({
  description,
  label,
  tone = "muted",
  value,
}: Metric) {
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-2 border px-3 py-3"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.border,
        borderRadius: dashboardTheme.radii.panel,
      }}
    >
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap items-start justify-between gap-2">
        <MetaText>{label}</MetaText>
        <StateIndicator tone={tone} />
      </View>
      <Text
        className="max-w-full min-w-0 break-words text-lg font-extrabold"
        selectable
        style={{ color: dashboardTheme.colors.textPrimary }}
      >
        {value}
      </Text>
      {description ? <SubtleText>{description}</SubtleText> : null}
    </View>
  );
}


export function HealthStrip({ items }: { items: HealthItem[] }) {
  const dashboardTheme = useThemeTokens();

  return (
    <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
      {items.map((item) => (
        <View
          className="min-w-0 flex-1 gap-1 border px-3 py-3"
          key={item.id}
          style={{
            flexBasis: 180,
            maxWidth: "100%",
            backgroundColor: dashboardTheme.colors.surface,
            borderColor: dashboardTheme.colors.borderSoft,
            borderRadius: dashboardTheme.radii.panel,
          }}
        >
          <View className="flex-row flex-wrap items-center justify-between gap-2">
            <MetaText>{item.label}</MetaText>
            <StateIndicator tone={item.tone} />
          </View>
          <Text
            className="max-w-full min-w-0 break-words text-sm font-bold"
            selectable
            style={{ color: dashboardTheme.colors.textPrimary }}
          >
            {item.value}
          </Text>
          {item.description ? (
            <SubtleText>{item.description}</SubtleText>
          ) : null}
        </View>
      ))}
    </View>
  );
}

export function EventTimeline({
  emptyLabel = "Aucun événement.",
  items,
}: {
  emptyLabel?: string;
  items: TimelineItem[];
}) {
  const tokens = useThemeTokens();

  if (items.length === 0) {
    return <EmptyState title={emptyLabel} />;
  }

  return (
    <View className="w-full max-w-full min-w-0 gap-2">
      {items.map((item) => (
        <DataRow key={item.id}>
          <View className="w-full max-w-full min-w-0 gap-2">
            <View className="w-full max-w-full min-w-0 flex-row flex-wrap items-start gap-2">
              <StateIndicator tone={item.tone} />
              <Text
                className="min-w-0 flex-1 text-sm font-bold"
                style={{ color: tokens.colors.textPrimary }}
              >
                {item.title}
              </Text>
            </View>
            {item.description ? (
              <SubtleText>{item.description}</SubtleText>
            ) : null}
            {item.meta ? <MetaText>{item.meta}</MetaText> : null}
          </View>
        </DataRow>
      ))}
    </View>
  );
}

export function TriPane({
  inspector,
  inspectorWidth = 320,
  isWide,
  main,
  rail,
  railWidth = 300,
  style,
}: TriPaneProps) {
  return (
    <View
      className="w-full max-w-full min-w-0 gap-3"
      style={[{ flexDirection: isWide ? "row" : "column" }, style]}
    >
      <View
        className="w-full max-w-full min-w-0 gap-3"
        style={
          isWide
            ? { flexBasis: railWidth, flexGrow: 0, flexShrink: 0 }
            : undefined
        }
      >
        {rail}
      </View>
      <View className="min-w-0 flex-1 gap-3">{main}</View>
      <View
        className="w-full max-w-full min-w-0 gap-3"
        style={
          isWide
            ? { flexBasis: inspectorWidth, flexGrow: 0, flexShrink: 0 }
            : undefined
        }
      >
        {inspector}
      </View>
    </View>
  );
}

export function FilterRail({
  children,
  description,
  title = "Filtres",
}: PropsWithChildren<{
  description?: ReactNode;
  title?: ReactNode;
}>) {
  return (
    <SectionPanel title={title} description={description}>
      <View className="w-full max-w-full min-w-0 gap-3">{children}</View>
    </SectionPanel>
  );
}

export function InspectorPanel({
  children,
  description,
  fields = [],
  title = "Détails",
}: PropsWithChildren<{
  description?: ReactNode;
  fields?: InspectorField[];
  title?: ReactNode;
}>) {
  const dashboardTheme = useThemeTokens();

  return (
    <SectionPanel title={title} description={description}>
      <View className="w-full max-w-full min-w-0 gap-3">
        {fields.length > 0 ? (
          <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
            {fields.map((field) => (
              <View
                className="min-w-[150px] flex-1 gap-1 border px-3 py-2"
                key={field.label}
                style={{
                  backgroundColor: dashboardTheme.colors.surface,
                  borderColor: dashboardTheme.colors.borderSoft,
                  borderRadius: dashboardTheme.radii.panel,
                }}
              >
                <View className="flex-row flex-wrap items-center justify-between gap-2">
                  <MetaText>{field.label}</MetaText>
                  {field.tone ? <StateIndicator tone={field.tone} /> : null}
                </View>
                <Text
                  className="max-w-full min-w-0 break-words text-sm font-bold"
                  selectable
                  style={{ color: dashboardTheme.colors.textPrimary }}
                >
                  {field.value}
                </Text>
              </View>
            ))}
          </View>
        ) : null}
        {children}
      </View>
    </SectionPanel>
  );
}

export function DataGrid<Row>({
  columns,
  density = "normal",
  emptyLabel = "Aucune donnée.",
  isLoading,
  keyExtractor,
  rows,
  selectedRowKey,
}: {
  columns: DataGridColumn<Row>[];
  density?: "normal" | "compact";
  emptyLabel?: string;
  isLoading?: boolean;
  keyExtractor: (row: Row, index: number) => string;
  rows: Row[];
  selectedRowKey?: string | null;
}) {
  const t = useThemeTokens();
  const { width } = useWindowDimensions();
  const compact = width < 1024;
  if (isLoading)
    return (
      <ActivityIndicator
        accessibilityLabel="Chargement du tableau"
        color={t.colors.textPrimary}
      />
    );
  if (!rows.length) return <EmptyState title={emptyLabel} />;
  const minimum = columns.reduce((sum, c) => sum + (c.minWidth ?? 140), 0);
  const cellStyle = (column: DataGridColumn<Row>) => ({
    flexGrow: 1,
    flexBasis: column.minWidth ?? 140,
    minWidth: column.minWidth ?? 140,
    padding: density === "compact" ? 12 : 16,
  });
  if (compact)
    return (
      <View className="gap-4" accessibilityRole="list">
        {rows.map((row, i) => (
          <DataRow
            role="listitem"
            key={keyExtractor(row, i)}
            style={{
              backgroundColor:
                selectedRowKey === keyExtractor(row, i)
                  ? t.colors.selection
                  : t.colors.surface,
            }}
          >
            {columns.map((column) => (
              <View key={column.key} className="min-w-0 gap-1 py-1">
                <MetaText>{column.label}</MetaText>
                {renderGridCell(column.render(row), t.colors.textPrimary)}
              </View>
            ))}
          </DataRow>
        ))}
      </View>
    );
  return (
    <ScrollView horizontal style={{ maxWidth: "100%" }}>
      <View
        role="table"
        style={{
          minWidth: minimum,
          flex: 1,
          borderWidth: 1,
          borderColor: t.colors.borderSoft,
          borderRadius: 8,
        }}
      >
        <View
          role="row"
          style={{ flexDirection: "row", backgroundColor: t.colors.background }}
        >
          {columns.map((column) => (
            <View
              role="columnheader"
              key={column.key}
              style={cellStyle(column)}
            >
              <MetaText>{column.label}</MetaText>
            </View>
          ))}
        </View>
        {rows.map((row, i) => (
          <View
            role="row"
            key={keyExtractor(row, i)}
            style={{
              flexDirection: "row",
              backgroundColor:
                selectedRowKey === keyExtractor(row, i)
                  ? t.colors.selection
                  : t.colors.surface,
              borderTopWidth: 1,
              borderColor: t.colors.borderSoft,
            }}
          >
            {columns.map((column) => (
              <View role="cell" key={column.key} style={cellStyle(column)}>
                {renderGridCell(column.render(row), t.colors.textPrimary)}
              </View>
            ))}
          </View>
        ))}
      </View>
    </ScrollView>
  );
}

export function WorkflowBoard({ stages }: { stages: WorkflowStage[] }) {
  return (
    <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-3">
      {stages.map((stage) => (
        <StageColumn key={stage.id} stage={stage} />
      ))}
    </View>
  );
}

export function StageColumn({ stage }: { stage: WorkflowStage }) {
  return (
    <View
      style={{ minWidth: 0, flexGrow: 1, flexBasis: 260, maxWidth: "100%" }}
    >
      <SectionPanel
        title={stage.title}
        description={stage.description}
        action={<StateIndicator tone={stage.tone} />}
      >
        <View className="w-full max-w-full min-w-0 gap-2">
          {stage.items.length === 0 ? (
            <EmptyState title="Aucune étape." />
          ) : (
            stage.items.map((step) => <RunStepRow key={step.id} step={step} />)
          )}
        </View>
      </SectionPanel>
    </View>
  );
}

export function RunStepRow({ step }: { step: WorkflowStep }) {
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-2 border px-3 py-2"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.borderSoft,
        borderRadius: dashboardTheme.radii.panel,
      }}
    >
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap items-start gap-2">
        <StateIndicator tone={step.tone} />
        <Text
          className="min-w-0 flex-1 text-sm font-bold"
          style={{ color: dashboardTheme.colors.textPrimary }}
        >
          {step.title}
        </Text>
      </View>
      {step.description ? <SubtleText>{step.description}</SubtleText> : null}
      {step.meta ? <MetaText>{step.meta}</MetaText> : null}
    </View>
  );
}

export function LogToolbar({
  actions,
  children,
  status,
}: PropsWithChildren<{
  actions?: CommandAction[];
  status?: ReactNode;
}>) {
  return (
    <CommandBar
      title="Console de l’opérateur"
      description="Filtres, session en direct et commandes de lecture."
      actions={actions}
      meta={status}
    >
      {children}
    </CommandBar>
  );
}

export function TerminalPanel({
  children,
  description,
  onContentSizeChange,
  onScroll,
  scrollEventThrottle,
  scrollRef,
  title = "Terminal",
}: PropsWithChildren<{
  description?: ReactNode;
  onContentSizeChange?: () => void;
  onScroll?: (event: NativeSyntheticEvent<NativeScrollEvent>) => void;
  scrollEventThrottle?: number;
  scrollRef?: RefObject<ScrollViewType | null>;
  title?: ReactNode;
}>) {
  const dashboardTheme = useThemeTokens();

  return (
    <SectionPanel title={title} description={description}>
      <ScrollView
        ref={scrollRef}
        className="w-full max-w-full min-w-0 border px-3 py-3"
        onContentSizeChange={onContentSizeChange}
        onScroll={onScroll}
        scrollEventThrottle={scrollEventThrottle}
        style={[
          terminalScrollStyle,
          {
            backgroundColor: dashboardTheme.colors.surface,
            borderColor: dashboardTheme.colors.border,
            borderRadius: dashboardTheme.radii.panel,
          },
        ]}
      >
        {children}
      </ScrollView>
    </SectionPanel>
  );
}

export function EmptyState({
  description,
  title,
}: {
  description?: ReactNode;
  title: ReactNode;
}) {
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-1 border px-3 py-3"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.borderSoft,
        borderRadius: dashboardTheme.radii.panel,
      }}
    >
      <MetaText>{title}</MetaText>
      {description ? <SubtleText>{description}</SubtleText> : null}
    </View>
  );
}

export function ErrorState({ message }: { message: string }) {
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 border px-3 py-3"
      style={{
        backgroundColor: tokens.colors.surface,
        borderColor: tokens.colors.dangerText,
        borderRadius: dashboardTheme.radii.panel,
      }}
    >
      <SubtleText style={{ color: tokens.colors.dangerText }}>
        {message}
      </SubtleText>
    </View>
  );
}

function renderGridCell(value: ReactNode, textColor: string) {
  if (typeof value === "string" || typeof value === "number") {
    return (
      <Text
        className="max-w-full min-w-0 break-words text-sm font-bold"
        selectable
        style={{ color: textColor }}
      >
        {value}
      </Text>
    );
  }

  return value;
}

const terminalScrollStyle =
  Platform.OS === "web"
    ? ({
        boxSizing: "border-box",
        maxHeight: "72vh",
        minHeight: 420,
        overflowX: "hidden",
        width: "100%",
      } as unknown as ViewStyle)
    : ({
        maxHeight: 640,
        minHeight: 420,
      } as ViewStyle);

export const terminalTextStyle =
  Platform.OS === "web"
    ? ({
        fontFamily:
          "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
        maxWidth: "100%",
        overflowWrap: "anywhere",
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
      } as unknown as TextStyle)
    : ({
        fontFamily: "monospace",
      } as TextStyle);

export { useThemeTokens };

export { DataRow };
export type * from "./workspaceTypes";
