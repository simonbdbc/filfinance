import type { ComponentProps } from "react";

import { TestRunHistoryCard } from "./TestRunHistoryCard";
import { TestRunControlCard } from "./TestRunControlCard";
import { TestRunTraceCard } from "./TestRunTraceCard";

type TestRunsPanelProps = {
  controlProps: ComponentProps<typeof TestRunControlCard>;
  traceProps: ComponentProps<typeof TestRunTraceCard>;
  historyProps: ComponentProps<typeof TestRunHistoryCard>;
};

export function TestRunsPanel({
  controlProps,
  traceProps,
  historyProps,
}: TestRunsPanelProps) {
  return (
    <>
      <TestRunControlCard {...controlProps} />
      <TestRunTraceCard {...traceProps} />
      <TestRunHistoryCard {...historyProps} />
    </>
  );
}
