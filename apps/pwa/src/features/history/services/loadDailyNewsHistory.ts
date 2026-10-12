import type { AuthSessionClient } from "@shared/api/authSession";
import type { TranslatedNewsItem, TranslationHistoryResponse } from "@shared/api/contracts";
import { cachedJsonGet } from "@shared/api/requestCache";
import { buildTranslationsHistoryUrl } from "@shared/api/urls";
import {
  dedupeRealFeedItemsBySourcePriority,
  mergeHistoryItemsNewestFirst,
  sortHistoryNewestFirst,
} from "@shared/lib/newsHistory";

type HistoryScope = "real_feed" | "test_run";
const realFeedSources = ["financial_juice", "financial_juice_browser"] as const;

export type DailyNewsHistory = {
  feedItems: TranslatedNewsItem[];
  testRunItems: TranslatedNewsItem[];
};

type LoadDailyNewsHistoryOptions = {
  wsUrl: string;
  authSession: AuthSessionClient | null;
  currentLocalDayKey: string;
  previousLocalDayKey: string;
  signal: AbortSignal;
};

async function loadHistoryScope(
  options: LoadDailyNewsHistoryOptions,
  historyScope: HistoryScope,
  source?: string,
) {
  let mergedItems: TranslatedNewsItem[] = [];

  for (const day of [options.currentLocalDayKey, options.previousLocalDayKey]) {
    let nextBeforeId: number | null = null;

    do {
      const payload: TranslationHistoryResponse = await cachedJsonGet<TranslationHistoryResponse>(
        buildTranslationsHistoryUrl(options.wsUrl, {
          day,
          historyScope,
          source,
          limit: 200,
          beforeId: nextBeforeId,
        }),
        {
          authSession: options.authSession,
          context: `historique ${historyScope}${source ? ` ${source}` : ""}`,
          signal: options.signal,
          ttlMs: 10_000,
        },
      );
      mergedItems = mergeHistoryItemsNewestFirst(mergedItems, payload.items);
      nextBeforeId = payload.has_more ? payload.next_before_id : null;
    } while (nextBeforeId && !options.signal.aborted);
  }

  return sortHistoryNewestFirst(mergedItems);
}

export async function loadDailyNewsHistory(
  options: LoadDailyNewsHistoryOptions,
): Promise<DailyNewsHistory> {
  const [feedSourceItems, testRunItems] = await Promise.all([
    Promise.all(
      realFeedSources.map((source) =>
        loadHistoryScope(options, "real_feed", source)
      ),
    ),
    loadHistoryScope(options, "test_run"),
  ]);
  const feedItems = dedupeRealFeedItemsBySourcePriority(sortHistoryNewestFirst(
    feedSourceItems.reduce<TranslatedNewsItem[]>(
      (mergedItems, sourceItems) =>
        mergeHistoryItemsNewestFirst(mergedItems, sourceItems),
      [],
    ),
  ));

  return { feedItems, testRunItems };
}
