import AsyncStorage from "@react-native-async-storage/async-storage";
import { useCallback, useEffect, useState } from "react";

import { normalizeImportance } from "@shared/lib/importance";
import {
  type CalendarFilterGroup,
  type CalendarFilters,
  type LocationFilter,
  emptyCalendarFilters,
  locationFilters,
} from "../lib/calendarFilters";

export type UsePersistentCalendarFiltersResult = {
  clearFilters: () => void;
  filters: CalendarFilters;
  hasActiveFilters: boolean;
  toggleFilter: (group: CalendarFilterGroup, value: string) => void;
};

export function usePersistentCalendarFilters(
  storageKey: string,
): UsePersistentCalendarFiltersResult {
  const [filters, setFilters] = useState<CalendarFilters>(emptyCalendarFilters);
  const [hasLoaded, setHasLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;

    void AsyncStorage.getItem(storageKey)
      .then((storedValue) => {
        if (cancelled) {
          return;
        }
        setFilters(parseStoredFilters(storedValue));
      })
      .catch(() => {
        if (!cancelled) {
          setFilters(emptyCalendarFilters);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setHasLoaded(true);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [storageKey]);

  useEffect(() => {
    if (!hasLoaded) {
      return;
    }
    void AsyncStorage.setItem(storageKey, JSON.stringify(filters)).catch(() => undefined);
  }, [filters, hasLoaded, storageKey]);

  const toggleFilter = useCallback((group: CalendarFilterGroup, value: string) => {
    setFilters((current) => {
      const currentValues = current[group];
      const nextValues = currentValues.includes(value)
        ? currentValues.filter((entry) => entry !== value)
        : [...currentValues, value];
      return {
        ...current,
        [group]: nextValues,
      };
    });
  }, []);

  const clearFilters = useCallback(() => {
    setFilters(emptyCalendarFilters);
  }, []);

  return {
    clearFilters,
    filters,
    hasActiveFilters: Object.values(filters).some((values) => values.length > 0),
    toggleFilter,
  };
}

function parseStoredFilters(value: string | null): CalendarFilters {
  if (!value) {
    return emptyCalendarFilters;
  }

  try {
    const parsed = JSON.parse(value) as Partial<CalendarFilters>;
    return {
      importance: arrayValue(parsed.importance).filter((entry) =>
        Boolean(normalizeImportance(entry)),
      ),
      location: arrayValue(parsed.location).filter((entry) =>
        locationFilters.includes(entry as LocationFilter),
      ),
      type: arrayValue(parsed.type),
    };
  } catch {
    return emptyCalendarFilters;
  }
}

function arrayValue(value: unknown) {
  return Array.isArray(value)
    ? value.filter((entry): entry is string => typeof entry === "string")
    : [];
}
