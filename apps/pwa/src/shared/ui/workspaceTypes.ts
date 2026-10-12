import type { ReactNode } from "react";
import type { ViewStyle } from "react-native";
import type { ThemeTokens } from "@shared/theme";

export type Tone = "critical" | "dark" | "info" | "muted" | "trace";

export type Metric = {
  description?: ReactNode;
  label: string;
  tone?: Tone;
  value: ReactNode;
};

export type HealthItem = Metric & {
  id: string;
};

export type CommandAction = {
  disabled?: boolean;
  label: string;
  onPress: () => void;
  variant?: "primary" | "secondary";
};

export type TimelineItem = {
  description?: ReactNode;
  id: string;
  meta?: ReactNode;
  tone?: Tone;
  title: ReactNode;
};

export type DataGridColumn<Row> = {
  key: string;
  label: string;
  minWidth?: number;
  render: (row: Row) => ReactNode;
};

export type InspectorField = {
  label: string;
  tone?: Tone;
  value: ReactNode;
};

export type WorkflowStep = {
  description?: ReactNode;
  id: string;
  meta?: ReactNode;
  tone?: Tone;
  title: ReactNode;
};

export type WorkflowStage = {
  description?: ReactNode;
  id: string;
  items: WorkflowStep[];
  tone?: Tone;
  title: string;
};

export type WorkspaceTheme = ThemeTokens;

export type GridDirection = "horizontal" | "vertical";

export type GridProps = {
  children: ReactNode;
  direction?: GridDirection;
  minItemWidth?: number;
  style?: ViewStyle;
};

export type GridAreaProps = {
  children: ReactNode;
  minWidth?: number;
  span?: "auto" | "full";
  style?: ViewStyle;
};

export type TriPaneProps = {
  inspector: ReactNode;
  inspectorWidth?: number;
  isWide: boolean;
  main: ReactNode;
  rail: ReactNode;
  railWidth?: number;
  style?: ViewStyle;
};
