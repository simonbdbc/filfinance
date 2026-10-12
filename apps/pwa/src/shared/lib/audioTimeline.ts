export type AudioTimelineItem = {
  id: number;
  timestamp?: string | null;
  created_at?: string | null;
};

export function audioTimelineTimestampValue(item: AudioTimelineItem) {
  return item.timestamp || item.created_at || "";
}

export function compareAudioTimelineItems(
  left: AudioTimelineItem,
  right: AudioTimelineItem,
) {
  const timestampComparison = compareAudioTimelineTimestampValues(
    audioTimelineTimestampValue(left),
    audioTimelineTimestampValue(right),
  );
  if (timestampComparison !== 0) {
    return timestampComparison;
  }

  return left.id - right.id;
}

function compareAudioTimelineTimestampValues(left: string, right: string) {
  const leftTime = Date.parse(left);
  const rightTime = Date.parse(right);
  if (Number.isFinite(leftTime) && Number.isFinite(rightTime) && leftTime !== rightTime) {
    return leftTime - rightTime;
  }

  return left.localeCompare(right);
}

export function pickNewestAudioTimelineItem(
  items: Array<AudioTimelineItem | null | undefined>,
) {
  return items.reduce<AudioTimelineItem | null>((newestItem, item) => {
    if (!item) {
      return newestItem;
    }

    if (!newestItem || compareAudioTimelineItems(newestItem, item) < 0) {
      return item;
    }

    return newestItem;
  }, null);
}

export function isAudioTimelineItemOlderThan(
  candidate: AudioTimelineItem,
  reference: AudioTimelineItem,
) {
  return compareAudioTimelineItems(candidate, reference) < 0;
}

export function shouldAdmitAudioTimelineItem(
  candidate: AudioTimelineItem,
  currentPlaybackItem: AudioTimelineItem | null,
  lastReceivedItem: AudioTimelineItem | null,
) {
  const referenceItem = pickNewestAudioTimelineItem([
    currentPlaybackItem,
    lastReceivedItem,
  ]);
  if (!referenceItem) {
    return true;
  }

  return !isAudioTimelineItemOlderThan(candidate, referenceItem);
}

export function mergeAudioTimelineItemsOldestFirst<T extends AudioTimelineItem>(
  existingItems: T[],
  nextItems: T[],
  maxItems: number,
) {
  const mergedItems = new Map<number, T>();

  for (const item of [...existingItems, ...nextItems]) {
    if (item.id <= 0) {
      continue;
    }

    mergedItems.set(item.id, item);
  }

  return [...mergedItems.values()]
    .sort(compareAudioTimelineItems)
    .slice(0, maxItems);
}
