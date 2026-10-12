import { View } from "react-native";

import { shortWeekdayLabel } from "@shared/lib/dateTime";
import { Button, ChevronIcon } from "@shared/ui";
import { CalendarFilterControls } from "./CalendarFilterControls";

type CalendarFiltersBlockContentProps = {
  days: string[];
  isExpanded: boolean;
  onExpandedChange: (isExpanded: boolean) => void;
  onSelectDay: (day: string) => void;
  options: Parameters<typeof CalendarFilterControls>[0]["options"];
  selectedDay: string;
  state: Parameters<typeof CalendarFilterControls>[0]["state"];
};

export function CalendarFiltersBlockContent({
  days,
  isExpanded,
  onExpandedChange,
  onSelectDay,
  options,
  selectedDay,
  state,
}: CalendarFiltersBlockContentProps) {
  return (
    <View className="w-full max-w-full min-w-0 gap-3">
      <View className="flex-row items-stretch gap-1">
        {days.map((day) => (
          <Button
            className="min-w-0 flex-1 px-1"
            selected={day === selectedDay}
            key={day}
            onPress={() => onSelectDay(day)}
            variant={day === selectedDay ? "primary" : "secondary"}
          >
            {shortWeekdayLabel(day).replace(" ", "\n")}
          </Button>
        ))}
      </View>

      <View className="w-full max-w-full min-w-0">
        <Button
          className="w-full"
          onPress={() => onExpandedChange(!isExpanded)}
          suffix={
            <ChevronIcon direction={isExpanded ? "up" : "down"} size={14} />
          }
        >
          Filtres
        </Button>
      </View>

      {isExpanded ? (
        <CalendarFilterControls options={options} state={state} />
      ) : null}
    </View>
  );
}
