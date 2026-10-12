import { openBrowserContext } from "./browser.js";
import { loadConfig, validateConfig } from "./config.js";
import { loadLocalEnvFilePath } from "./env-file.js";
import {
  expandNewsHistoryFromPage,
  extractNewsFromPageWithDiagnostics,
} from "./extract-news.js";
import {
  loadControls,
  publishNewsBatch,
  publishStatusToTargets,
} from "./publish.js";
import { RecentlyPublishedIds } from "./recently-published.js";
import { ensureFinancialJuiceSession } from "./session.js";

loadLocalEnvFilePath();

const config = loadConfig();
const validationErrors = validateConfig(config);

if (validationErrors.length > 0) {
  for (const error of validationErrors) {
    console.error(`[financialjuice-browser] ${error}`);
  }
  process.exit(1);
}

if (!config.enabled) {
  console.log("[financialjuice-browser] collector disabled by NEWS_FINANCIAL_JUICE_BROWSER_ENABLED=false");
  process.exit(0);
}

console.log(
  `[financialjuice-browser] collector enabled headless=${config.headless} profile=${config.userDataDir} targets=${config.targets.map((target) => target.name).join(",")}`,
);

let stopping = false;
process.once("SIGINT", () => {
  stopping = true;
});
process.once("SIGTERM", () => {
  stopping = true;
});

async function runCollector(activeConfig) {
  let context = null;
  let page = null;
  const recentlyPublishedByTarget = new Map();
  let historyBackfill = freshHistoryBackfillState();
  let browserBackoffUntil = 0;
  let browserBackoffMessage = null;

  try {
    await logStatusPublishResults(
      "startup",
      publishStatusToTargets(activeConfig, "stopped", "collector process starting"),
    );

    while (!stopping) {
      const controlResults = await loadControls(activeConfig);
      logControlErrors(controlResults);

      const availableControls = controlResults.filter((result) => result.control);
      if (availableControls.length === 0) {
        await sleep(activeConfig.pollIntervalMs);
        continue;
      }

      const fallbackControls = availableControls.filter((result) => result.control.fallback_active);
      const sleepMs = nextControlPollInterval(activeConfig, availableControls);

      if (fallbackControls.length === 0) {
        if (context) {
          await context.close();
          context = null;
          page = null;
          historyBackfill = freshHistoryBackfillState();
        }
        browserBackoffUntil = 0;
        browserBackoffMessage = null;
        const message = formatIdleMessage(availableControls);
        await logStatusPublishResults(
          "idle",
          publishStatusToTargets(activeConfig, "stopped", message),
        );
        console.log(`[financialjuice-browser] idle: ${message}`);
        await sleep(sleepMs);
        continue;
      }

      if (Date.now() < browserBackoffUntil) {
        const retryInMs = Math.max(browserBackoffUntil - Date.now(), 0);
        await logStatusPublishResults(
          "browser-backoff",
          publishStatusForFallbackTargets(
            activeConfig,
            fallbackControls.map((controlResult) => controlResult.target),
            "blocked",
            `${browserBackoffMessage ?? "FinancialJuice browser is backing off"}; retry_in_ms=${retryInMs}`,
          ),
        );
        await sleep(Math.min(sleepMs, Math.max(retryInMs, 1_000)));
        continue;
      }

      if (!context) {
        context = await openBrowserContext(activeConfig);
        page = await context.newPage();
      }

      const result = await runOnce(
        activeConfig,
        page,
        recentlyPublishedByTarget,
        historyBackfill,
        fallbackControls.map((controlResult) => controlResult.target),
      );
      if (result.browserBackoffMs) {
        browserBackoffUntil = Date.now() + result.browserBackoffMs;
        browserBackoffMessage = result.message ?? null;
      } else {
        browserBackoffUntil = 0;
        browserBackoffMessage = null;
      }
      await sleep(result.retryAfterMs ?? sleepMs);
    }
  } finally {
    await logStatusPublishResults(
      "shutdown",
      publishStatusToTargets(activeConfig, "stopped", "collector process stopping"),
    );
    if (context) {
      await context.close();
    }
  }
}

async function runOnce(
  activeConfig,
  page,
  recentlyPublishedByTarget,
  historyBackfill,
  fallbackTargets,
) {
  try {
    const session = await ensureFinancialJuiceSession(page, activeConfig);
    if (!session.ok) {
      await logStatusPublishResults(
        "session",
        publishStatusForFallbackTargets(
          activeConfig,
          fallbackTargets,
          session.status,
          session.message,
        ),
      );
      console.warn(`[financialjuice-browser] ${session.message}`);
      return {
        browserBackoffMs: session.status === "blocked"
          ? activeConfig.rateLimitBackoffMs
          : null,
        retryAfterMs: activeConfig.pollIntervalMs,
        message: session.message,
      };
    }

    const historyExpansion = await maybeExpandNewsHistory(activeConfig, page, historyBackfill);
    const extraction = await extractNewsFromPageWithDiagnostics(page, {
      ...activeConfig,
      maxBatchSize: Number.MAX_SAFE_INTEGER,
    });
    const windowedEvents = filterEventsForLiveHistoryWindow(extraction.events, activeConfig);
    const windowSkipped = extraction.events.length - windowedEvents.length;
    const recentSkipped = countEventsKnownByEveryFallbackTarget(
      windowedEvents,
      recentlyPublishedByTarget,
      fallbackTargets,
    );

    if (windowedEvents.length === 0 || recentSkipped === windowedEvents.length) {
      await logStatusPublishResults(
        "empty",
        publishStatusForFallbackTargets(
          activeConfig,
          fallbackTargets,
          "ok",
          `FinancialJuice #mainFeed available; ${formatExtractionDiagnostics(extraction.diagnostics, recentSkipped, windowSkipped)}${formatHistoryExpansion(historyExpansion)}`,
        ),
      );
      return {};
    }

    const result = await publishNewsBatchesToTargets(
      activeConfig,
      windowedEvents,
      fallbackTargets,
      recentlyPublishedByTarget,
    );
    await logStatusPublishResults(
      "published",
      publishStatusForFallbackTargets(
        activeConfig,
        fallbackTargets,
        result.errors > 0 ? "degraded" : "ok",
        `published=${result.accepted} dropped=${result.dropped} fresh=${result.fresh} targets=${formatTargetSummaries(result.targets)} ${formatExtractionDiagnostics(extraction.diagnostics, recentSkipped, windowSkipped)}${formatHistoryExpansion(historyExpansion)}`,
      ),
    );
    console.log(`[financialjuice-browser] published ${result.fresh} events targets=${formatTargetSummaries(result.targets)}`);
    return {};
  } catch (error) {
    await logStatusPublishResults(
      "poll-error",
      publishStatusForFallbackTargets(activeConfig, fallbackTargets, "degraded", error.message),
    );
    console.error(`[financialjuice-browser] polling failed: ${error.stack ?? error.message}`);
    return {};
  }
}

function freshHistoryBackfillState() {
  return {
    lastRunAt: 0,
    dayKey: null,
  };
}

async function maybeExpandNewsHistory(activeConfig, page, historyBackfill) {
  const dayKey = localDayKey(new Date(), activeConfig.historyTimezone);
  const now = Date.now();
  if (
    historyBackfill.dayKey === dayKey
    && now - historyBackfill.lastRunAt < activeConfig.historyRefreshMs
  ) {
    return null;
  }

  const result = await expandNewsHistoryFromPage(page, activeConfig);
  historyBackfill.dayKey = dayKey;
  historyBackfill.lastRunAt = Date.now();
  return result;
}

async function publishNewsBatchesToTargets(
  activeConfig,
  events,
  fallbackTargets,
  recentlyPublishedByTarget,
) {
  const summaries = [];
  let accepted = 0;
  let dropped = 0;
  let fresh = 0;
  let errors = 0;

  for (const target of fallbackTargets) {
    const tracker = recentlyPublishedTracker(recentlyPublishedByTarget, target);
    const freshEvents = events.filter((event) => !tracker.has(event.external_id));
    if (freshEvents.length === 0) {
      summaries.push({ target, accepted: 0, dropped: 0, fresh: 0, error: null });
      continue;
    }

    try {
      const targetConfig = { ...target, maxBatchSize: activeConfig.maxBatchSize };
      let targetAccepted = 0;
      let targetDropped = 0;
      for (const batch of chunkEvents(freshEvents, activeConfig.maxBatchSize)) {
        const result = await publishNewsBatch(targetConfig, batch);
        targetAccepted += result.accepted ?? batch.length;
        targetDropped += result.dropped ?? 0;
      }
      tracker.rememberAll(freshEvents);
      accepted += targetAccepted;
      dropped += targetDropped;
      fresh += freshEvents.length;
      summaries.push({
        target,
        accepted: targetAccepted,
        dropped: targetDropped,
        fresh: freshEvents.length,
        error: null,
      });
    } catch (error) {
      errors += 1;
      summaries.push({
        target,
        accepted: 0,
        dropped: 0,
        fresh: freshEvents.length,
        error,
      });
      console.error(`[financialjuice-browser] failed to publish news to target=${target.name}: ${error.message}`);
    }
  }

  return { accepted, dropped, fresh, errors, targets: summaries };
}

function chunkEvents(events, chunkSize) {
  const chunks = [];
  for (let index = 0; index < events.length; index += chunkSize) {
    chunks.push(events.slice(index, index + chunkSize));
  }
  return chunks;
}

function filterEventsForLiveHistoryWindow(events, activeConfig) {
  const timezone = activeConfig.historyTimezone ?? "Europe/Paris";
  const today = localDayKey(new Date(), timezone);
  const yesterday = localDayKey(daysAgo(1), timezone);
  return events.filter((event) => {
    const dayKey = localDayKeyFromTimestamp(event.published_at, timezone);
    return dayKey === today || dayKey === yesterday;
  });
}

function recentlyPublishedTracker(recentlyPublishedByTarget, target) {
  if (!recentlyPublishedByTarget.has(target.name)) {
    recentlyPublishedByTarget.set(target.name, new RecentlyPublishedIds(10_000));
  }
  return recentlyPublishedByTarget.get(target.name);
}

function countEventsKnownByEveryFallbackTarget(events, recentlyPublishedByTarget, fallbackTargets) {
  if (fallbackTargets.length === 0) {
    return 0;
  }
  return events.filter((event) => fallbackTargets.every((target) => {
    const tracker = recentlyPublishedByTarget.get(target.name);
    return tracker?.has(event.external_id) ?? false;
  })).length;
}

function nextControlPollInterval(activeConfig, controlResults) {
  const intervals = controlResults
    .map((result) => result.control?.poll_interval_ms)
    .filter((interval) => Number.isSafeInteger(interval) && interval > 0);
  return intervals.length > 0
    ? Math.min(...intervals)
    : activeConfig.pollIntervalMs;
}

function formatIdleMessage(controlResults) {
  return controlResults
    .map((result) => `${result.target.name}=${result.control.message ?? "fallback inactive"}`)
    .join("; ");
}

function logControlErrors(controlResults) {
  for (const result of controlResults) {
    if (result.error) {
      console.error(
        `[financialjuice-browser] backend control endpoint unavailable target=${result.target.name}: ${result.error.message}`,
      );
    }
  }
}

async function logStatusPublishResults(context, promise) {
  const results = await promise;
  for (const result of results) {
    if (result.error) {
      console.error(
        `[financialjuice-browser] failed to publish ${context} status target=${result.target.name}: ${result.error.message}`,
      );
    }
  }
  return results;
}

async function publishStatusForFallbackTargets(config, fallbackTargets, status, message) {
  const activeNames = new Set(fallbackTargets.map((target) => target.name));
  const inactiveTargets = config.targets.filter((target) => !activeNames.has(target.name));
  const results = [];

  if (fallbackTargets.length > 0) {
    results.push(
      ...(await publishStatusToTargets(config, status, message, fallbackTargets)),
    );
  }
  if (inactiveTargets.length > 0) {
    results.push(
      ...(await publishStatusToTargets(
        config,
        "stopped",
        `fallback inactive; shared collector active for ${[...activeNames].join(",") || "none"}`,
        inactiveTargets,
      )),
    );
  }

  return results;
}

function formatTargetSummaries(summaries) {
  return summaries.map((summary) => {
    if (summary.error) {
      return `${summary.target.name}:error=${summary.error.message}`;
    }
    return `${summary.target.name}:published=${summary.accepted},dropped=${summary.dropped},fresh=${summary.fresh}`;
  }).join(";");
}

function daysAgo(days) {
  const date = new Date();
  date.setUTCDate(date.getUTCDate() - days);
  return date;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function formatExtractionDiagnostics(diagnostics, recentSkipped, windowSkipped = 0) {
  const primary = diagnostics.source === "text" && diagnostics.fallbackText
    ? diagnostics.fallbackText
    : diagnostics.dom;
  const parts = [
    `source=${diagnostics.source}`,
    `candidates=${primary.candidates}`,
    `normalized=${primary.normalized}`,
    `window_skipped=${windowSkipped}`,
    `recent_skipped=${recentSkipped}`,
  ];
  const rejected = formatRejectedDiagnostics(primary.rejected);
  if (rejected) {
    parts.push(`rejected=${rejected}`);
  }
  if (diagnostics.source === "text" && diagnostics.dom) {
    parts.push(`dom_candidates=${diagnostics.dom.candidates}`);
    parts.push(`dom_rejected=${formatRejectedDiagnostics(diagnostics.dom.rejected) || "0"}`);
  }
  return parts.join(" ");
}

function formatHistoryExpansion(historyExpansion) {
  if (!historyExpansion) {
    return "";
  }
  return [
    "",
    `history_pages=${historyExpansion.pagesLoaded}`,
    `history_stop=${historyExpansion.stopReason}`,
    `history_dom=${historyExpansion.domItems}`,
    `history_normalized=${historyExpansion.normalized}`,
    `history_oldest=${historyExpansion.oldestPublishedAt ?? "none"}`,
  ].join(" ");
}

function localDayKey(date, timezone = "Europe/Paris") {
  const parts = new Intl.DateTimeFormat("en", {
    timeZone: timezone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const part = (type) => parts.find((entry) => entry.type === type)?.value;
  return `${part("year")}-${part("month")}-${part("day")}`;
}

function localDayKeyFromTimestamp(value, timezone = "Europe/Paris") {
  if (!value) {
    return null;
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return localDayKey(date, timezone);
}

function formatRejectedDiagnostics(rejected) {
  return Object.entries(rejected ?? {})
    .filter(([, count]) => count > 0)
    .map(([reason, count]) => `${reason}:${count}`)
    .join(",");
}

await runCollector(config);
