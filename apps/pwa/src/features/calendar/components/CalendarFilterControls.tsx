import { View } from "react-native";

import { useThemeTokens } from "@shared/theme";
import { Button } from "@shared/ui/Button";
import { MetaText, SubtleText } from "@shared/ui/Typography";
import type {
  CalendarFilterGroup,
  CalendarFilterOption,
} from "../lib/calendarFilters";
import type { UsePersistentCalendarFiltersResult } from "../hooks/usePersistentCalendarFilters";

export function CalendarFilterControls({
  options,
  state,
}: {
  options: Record<CalendarFilterGroup, CalendarFilterOption[]>;
  state: UsePersistentCalendarFiltersResult;
}) {
  return (
    <View className="gap-3">
      <CalendarFilterGroupControls
        group="importance"
        label="Importance"
        options={options.importance}
        state={state}
      />
      <CalendarFilterGroupControls
        group="type"
        label="Type"
        options={options.type}
        state={state}
      />
      <CalendarFilterGroupControls
        group="location"
        label="Localisation"
        options={options.location}
        state={state}
      />
      {state.hasActiveFilters ? (
        <View className="flex-row flex-wrap gap-2">
          <Button className="min-w-[120px]" onPress={state.clearFilters}>
            Réinitialiser
          </Button>
        </View>
      ) : null}
    </View>
  );
}

function CalendarFilterGroupControls({
  group,
  label,
  options,
  state,
}: {
  group: CalendarFilterGroup;
  label: string;
  options: CalendarFilterOption[];
  state: UsePersistentCalendarFiltersResult;
}) {
  const tokens = useThemeTokens();
  const designTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-2 border px-3 py-3"
      style={{
        backgroundColor: tokens.colors.surface,
        borderColor: tokens.colors.border,
        borderRadius: designTheme.radii.panel,
      }}
    >
      <MetaText>{label}</MetaText>
      {options.length === 0 ? (
        <SubtleText>Aucune option disponible.</SubtleText>
      ) : (
        <View className="flex-row flex-wrap gap-2">
          {options.map((option) => {
            const isSelected = state.filters[group].includes(option.value);
            return (
              <Button
                key={option.value}
                className="min-w-[104px]"
                onPress={() => state.toggleFilter(group, option.value)}
                selected={isSelected}
                variant={isSelected ? "primary" : "secondary"}
              >
                {option.label} ({option.count})
              </Button>
            );
          })}
        </View>
      )}
    </View>
  );
}
