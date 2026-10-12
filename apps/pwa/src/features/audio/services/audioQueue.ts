import type { TranslatedNewsItem } from "@shared/api/contracts";
import {
  MAX_RECENT_COMPLETED_AUDIO_IDS,
  RECENT_AUDIO_DEDUP_WINDOW_MS,
} from "@shared/config/clientConfig";
import {
  type AudioTimelineItem,
  compareAudioTimelineItems,
  shouldAdmitAudioTimelineItem,
} from "@shared/lib/audioTimeline";

export function audioTimelineItemFromNewsItem(item: TranslatedNewsItem): AudioTimelineItem {
  return {
    id: item.id,
    timestamp: item.timestamp,
    created_at: item.created_at,
  };
}

export function shouldAdmitAudioItem(
  candidate: AudioTimelineItem,
  currentPlaybackItem: AudioTimelineItem | null,
  lastReceivedItem: AudioTimelineItem | null,
) {
  return shouldAdmitAudioTimelineItem(
    candidate,
    currentPlaybackItem,
    lastReceivedItem,
  );
}

export function nextLastReceivedAudioTimelineItem(
  lastReceivedItem: AudioTimelineItem | null,
  nextItem: AudioTimelineItem,
) {
  if (!lastReceivedItem || compareAudioTimelineItems(lastReceivedItem, nextItem) < 0) {
    return nextItem;
  }

  return lastReceivedItem;
}

export function pruneRecentCompletedAudioIds(
  recentCompletedAudioIds: Map<number, number>,
  now = Date.now(),
) {
  for (const [translationId, completedAt] of recentCompletedAudioIds.entries()) {
    if (now - completedAt > RECENT_AUDIO_DEDUP_WINDOW_MS) {
      recentCompletedAudioIds.delete(translationId);
    }
  }

  while (recentCompletedAudioIds.size > MAX_RECENT_COMPLETED_AUDIO_IDS) {
    const oldestEntry = recentCompletedAudioIds.keys().next();
    if (oldestEntry.done) {
      break;
    }
    recentCompletedAudioIds.delete(oldestEntry.value);
  }
}
