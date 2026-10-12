#!/usr/bin/env node

import { spawn, execFile } from "child_process";
import { createWriteStream, existsSync, mkdirSync, readFileSync } from "fs";
import os from "os";
import { dirname, resolve } from "path";
import { performance } from "perf_hooks";
import { fileURLToPath } from "url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");

loadEnvFile(
  process.env.FILFINANCE_PROD_LOAD_ENV_FILE ||
    process.env.FILFINANCE_ENV_FILE ||
    resolve(repoRoot, ".env.local"),
);

const args = new Set(process.argv.slice(2));

if (args.has("--help") || args.has("-h")) {
  printHelp();
  process.exit(0);
}

const config = {
  dryRun: args.has("--dry-run") || envBool("FILFINANCE_PROD_LOAD_DRY_RUN", false),
  confirm: envString("FILFINANCE_PROD_LOAD_CONFIRM", ""),
  profile: envString("FILFINANCE_PROD_LOAD_PROFILE", "prod-perf"),
  runId: envString("FILFINANCE_PROD_LOAD_RUN_ID", `prod-load-${timestampSlug()}`),
  apiBaseUrl: envString("FILFINANCE_LOAD_TEST_API_BASE_URL", "https://api.filfinance.fr"),
  backendWsUrl: envString("FILFINANCE_LOAD_TEST_WS_URL", "wss://api.filfinance.fr/ws/news"),
  hlsBaseUrl: normalizeOptionalBaseUrl(envString("FILFINANCE_LOAD_TEST_HLS_BASE_URL", "")),
  allowedHlsHosts: envString("FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS", ""),
  edgeWsUrl: envString("FILFINANCE_EDGE_WS_URL", "wss://live.filfinance.fr/ws/live"),
  edgePublishUrl: envString(
    "FILFINANCE_EDGE_PUBLISH_URL",
    "https://live.filfinance.fr/publish/live",
  ),
  tiers: envIntList(
    "FILFINANCE_PROD_LOAD_TIERS",
    defaultTiersForProfile(envString("FILFINANCE_PROD_LOAD_PROFILE", "prod-perf")),
  ),
  warmupTier: envInt("FILFINANCE_PROD_LOAD_WARMUP_TIER", 50),
  baselineTier: envInt("FILFINANCE_PROD_LOAD_BASELINE_TIER", 100),
  warmupDurationMs: envInt("FILFINANCE_PROD_LOAD_WARMUP_DURATION_MS", 60_000),
  tierDurationMs: envInt("FILFINANCE_PROD_LOAD_DURATION_MS", 120_000),
  runtimeProbeIntervalMs: envInt("FILFINANCE_PROD_LOAD_RUNTIME_PROBE_INTERVAL_MS", 10_000),
  testRunIntervalMs: envInt("FILFINANCE_PROD_LOAD_TEST_RUN_INTERVAL_MS", 15_000),
  hlsErrorThreshold: envFloat("FILFINANCE_PROD_LOAD_HLS_ERROR_THRESHOLD", 0.1),
  hlsLatencyMultiplier: envFloat("FILFINANCE_PROD_LOAD_HLS_LATENCY_MULTIPLIER", 1.2),
  confirmLatencyCrossing: envBool("FILFINANCE_PROD_LOAD_CONFIRM_LATENCY_CROSSING", true),
  generatorLoadRatioLimit: envFloat("FILFINANCE_PROD_LOAD_GENERATOR_LOAD_RATIO_LIMIT", 0.85),
  generatorFreeMemoryRatioLimit: envFloat(
    "FILFINANCE_PROD_LOAD_GENERATOR_FREE_MEMORY_RATIO_LIMIT",
    0.1,
  ),
  outputDir: envString(
    "FILFINANCE_PROD_LOAD_OUTPUT_DIR",
    resolve(os.tmpdir(), "filfinance-load-tests"),
  ),
  calendarDayTiers: envIntList("FILFINANCE_PROD_LOAD_CALENDAR_DAY_TIERS", "100,500"),
  calendarWeekAfterTier: envOptionalInt("FILFINANCE_PROD_LOAD_CALENDAR_WEEK_AFTER_TIER", 500),
  calendarLocalDay: envString("FILFINANCE_PROD_LOAD_CALENDAR_LOCAL_DAY", ""),
  calendarReferenceDay: envString("FILFINANCE_PROD_LOAD_CALENDAR_REFERENCE_DAY", ""),
  calendarForce: envBool("FILFINANCE_PROD_LOAD_CALENDAR_FORCE", true),
  combinedReportGraceMs: envInt("FILFINANCE_PROD_LOAD_COMBINED_REPORT_GRACE_MS", 2_000),
  combinedMaxOverrunMs: envInt("FILFINANCE_PROD_LOAD_COMBINED_MAX_OVERRUN_MS", 60_000),
  enableTestRuns: envBool("FILFINANCE_PROD_LOAD_ENABLE_TEST_RUNS", true),
  enableCalendar: envBool("FILFINANCE_PROD_LOAD_ENABLE_CALENDAR", true),
  enableSystemSnapshots: envBool("FILFINANCE_PROD_LOAD_ENABLE_SYSTEM_SNAPSHOTS", true),
  requestTimeoutMs: envInt("FILFINANCE_PROD_LOAD_REQUEST_TIMEOUT_MS", 10_000),
};

const runDir = resolve(config.outputDir, config.runId);
const jsonlPath = resolve(runDir, "events.jsonl");
const summaryPath = resolve(runDir, "summary.json");
let outputStream = null;
const tierResults = [];
let baselineP95Ms = null;
let stopReason = null;
let lastStableTier = null;
let calendarWeekTriggered = false;
let calendarLastStableDayTriggered = false;

main().catch((error) => {
  emit(
    {
      stage: "prod_full_pipeline_load_fatal",
      error: errorMessage(error),
    },
    { console: true },
  );
  outputStream?.end();
  process.exitCode = 1;
});

async function main() {
  validateConfig();

  if (!config.dryRun) {
    mkdirSync(runDir, { recursive: true });
    outputStream = createWriteStream(jsonlPath, { flags: "a" });
  }

  const branch = await currentGitBranch();
  const commit = await currentGitCommit();
  const accessToken = config.dryRun ? "" : await resolveAccessToken();

  emit({
    stage: "prod_full_pipeline_load_start",
    run_id: config.runId,
    profile: config.profile,
    branch,
    commit,
    dry_run: config.dryRun,
    urls: {
      api_base_url: config.apiBaseUrl,
      backend_ws_url: redactUrl(config.backendWsUrl),
      hls_base_url: config.hlsBaseUrl,
      allowed_hls_hosts: config.allowedHlsHosts,
      edge_ws_url: redactUrl(config.edgeWsUrl),
      edge_publish_url: redactUrl(config.edgePublishUrl),
    },
    thresholds: {
      hls_error_threshold: config.hlsErrorThreshold,
      hls_latency_multiplier: config.hlsLatencyMultiplier,
      generator_load_ratio_limit: config.generatorLoadRatioLimit,
      generator_free_memory_ratio_limit: config.generatorFreeMemoryRatioLimit,
    },
    tiers: config.tiers,
    output: config.dryRun ? null : { run_dir: runDir, events_jsonl: jsonlPath, summary: summaryPath },
  });

  await recordSystemSnapshot("campaign_start");
  if (!config.dryRun) {
    await recordRuntimeProbe("campaign_start", accessToken);
  }

  if (config.dryRun) {
    emit({
      stage: "prod_full_pipeline_load_dry_run_plan",
      planned_tiers: config.tiers.map((clients, index) => ({
        clients,
        duration_ms:
          clients === config.warmupTier && index === 0
            ? config.warmupDurationMs
            : config.tierDurationMs,
        warmup: clients === config.warmupTier && index === 0,
        calendar_day: shouldTriggerCalendarDay(clients),
        calendar_week: shouldTriggerCalendarWeek(clients),
        test_runs_enabled: config.enableTestRuns,
      })),
    });
    await finishSummary(branch, commit);
    return;
  }

  for (let index = 0; index < config.tiers.length; index += 1) {
    const clients = config.tiers[index];
    const warmup = clients === config.warmupTier && index === 0;
    const durationMs = warmup ? config.warmupDurationMs : config.tierDurationMs;
    const result = await executeTier({
      clients,
      durationMs,
      label: warmup ? "warmup" : "primary",
      accessToken,
      warmup,
      allowCalendar: true,
    });

    tierResults.push(result);

    if (warmup) {
      continue;
    }

    if (clients === config.baselineTier && result.metrics.hls_audio_delivery_p95_ms !== null) {
      baselineP95Ms = result.metrics.hls_audio_delivery_p95_ms;
      emit({
        stage: "prod_full_pipeline_load_baseline_selected",
        clients,
        baseline_p95_ms: baselineP95Ms,
      });
    }

    const evaluation = evaluateTier(result);
    result.evaluation = evaluation;
    emit({ stage: "prod_full_pipeline_load_tier_evaluation", evaluation, result: summarizeTier(result) });

    if (evaluation.generator_limited) {
      result.capacity_status = "generator_limited";
    } else if (!evaluation.threshold_crossed) {
      lastStableTier = clients;
      result.capacity_status = "stable";
    }

    if (evaluation.stop_now) {
      stopReason = evaluation.reason;
      break;
    }

    if (evaluation.requires_latency_confirmation) {
      const confirmation = await executeTier({
        clients,
        durationMs,
        label: "latency_confirmation",
        accessToken,
        warmup: false,
        allowCalendar: false,
      });
      const confirmationEvaluation = evaluateTier(confirmation, { latencyConfirmation: true });
      confirmation.evaluation = confirmationEvaluation;
      tierResults.push(confirmation);
      emit({
        stage: "prod_full_pipeline_load_latency_confirmation",
        evaluation: confirmationEvaluation,
        result: summarizeTier(confirmation),
      });

      if (confirmationEvaluation.threshold_crossed) {
        stopReason = confirmationEvaluation.reason;
        break;
      }
    }
  }

  if (
    config.enableCalendar &&
    lastStableTier !== null &&
    !calendarLastStableDayTriggered &&
    !config.calendarDayTiers.includes(lastStableTier)
  ) {
    await triggerCalendarDay(accessToken, {
      reasonSuffix: `last_stable_${lastStableTier}`,
      clients: lastStableTier,
    });
    calendarLastStableDayTriggered = true;
  }

  await recordRuntimeProbe("campaign_end", accessToken);
  await recordSystemSnapshot("campaign_end");
  await finishSummary(branch, commit);
}

async function executeTier({ clients, durationMs, label, accessToken, warmup, allowCalendar }) {
  emit({
    stage: "prod_full_pipeline_load_tier_start",
    label,
    clients,
    duration_ms: durationMs,
    warmup,
  });

  await recordSystemSnapshot(`tier_${clients}_${label}_start`);
  await recordRuntimeProbe(`tier_${clients}_${label}_start`, accessToken);

  if (allowCalendar && config.enableCalendar && shouldTriggerCalendarDay(clients)) {
    await triggerCalendarDay(accessToken, { reasonSuffix: `tier_${clients}`, clients });
  }
  if (allowCalendar && config.enableCalendar && shouldTriggerCalendarWeek(clients)) {
    await triggerCalendarWeek(accessToken, { reasonSuffix: `tier_${clients}`, clients });
    calendarWeekTriggered = true;
  }

  const tierStartedAt = new Date().toISOString();
  const processStart = process.resourceUsage();
  const loadStart = os.loadavg();
  const memoryStart = process.memoryUsage();
  const injector = config.enableTestRuns
    ? await startTestRunInjector(accessToken, clients, durationMs)
    : null;
  const probeLoop = startRuntimeProbeLoop(accessToken, `tier_${clients}_${label}`);

  const combined = await runCombinedLoadTest(clients, durationMs, label, accessToken);

  const runtimeProbeReport = await probeLoop.stop();
  const testRunReport = injector ? await injector.stop() : null;
  const processEnd = process.resourceUsage();
  const memoryEnd = process.memoryUsage();
  const loadEnd = os.loadavg();
  const elapsedMs = Date.now() - Date.parse(tierStartedAt);

  await recordRuntimeProbe(`tier_${clients}_${label}_end`, accessToken);
  const systemEnd = await recordSystemSnapshot(`tier_${clients}_${label}_end`);

  const metrics = deriveTierMetrics(combined);
  const generator = deriveGeneratorCost({
    elapsedMs,
    processStart,
    processEnd,
    memoryStart,
    memoryEnd,
    loadStart,
    loadEnd,
    systemEnd,
  });

  const result = {
    stage: "prod_full_pipeline_load_tier_result",
    run_id: config.runId,
    label,
    clients,
    duration_ms: durationMs,
    started_at: tierStartedAt,
    finished_at: new Date().toISOString(),
    warmup,
    combined,
    metrics,
    generator,
    test_runs: testRunReport,
    runtime_probes: runtimeProbeReport,
  };

  emit({ ...result, combined: summarizeCombined(combined) });
  return result;
}

function evaluateTier(result, options = {}) {
  const hlsErrorRate = result.metrics.hls_error_rate;
  const hlsP95 = result.metrics.hls_audio_delivery_p95_ms;
  const latencyLimit = baselineP95Ms === null ? null : round(baselineP95Ms * config.hlsLatencyMultiplier);
  const errorCrossed = hlsErrorRate !== null && hlsErrorRate >= config.hlsErrorThreshold;
  const latencyCrossed = latencyLimit !== null && hlsP95 !== null && hlsP95 >= latencyLimit;
  const generatorLimited = result.generator.limited;

  let reason = null;
  if (errorCrossed) {
    reason = "hls_error_rate_threshold";
  } else if (latencyCrossed) {
    reason = "hls_p95_latency_threshold";
  } else if (generatorLimited) {
    reason = "generator_limited";
  }

  return {
    clients: result.clients,
    label: result.label,
    baseline_p95_ms: baselineP95Ms,
    latency_limit_ms: latencyLimit,
    hls_audio_delivery_p95_ms: hlsP95,
    hls_error_rate: hlsErrorRate,
    threshold_crossed: errorCrossed || latencyCrossed,
    error_threshold_crossed: errorCrossed,
    latency_threshold_crossed: latencyCrossed,
    generator_limited: generatorLimited,
    reason,
    requires_latency_confirmation:
      latencyCrossed && config.confirmLatencyCrossing && !options.latencyConfirmation,
    stop_now:
      errorCrossed ||
      (latencyCrossed && (!config.confirmLatencyCrossing || options.latencyConfirmation)),
  };
}

async function runCombinedLoadTest(clients, durationMs, label, accessToken) {
  const startedAtMs = performance.now();
  const env = {
    ...process.env,
    ...(accessToken ? { FILFINANCE_LOAD_TEST_ACCESS_TOKEN: accessToken } : {}),
    FILFINANCE_LOAD_TEST_API_BASE_URL: config.apiBaseUrl,
    FILFINANCE_LOAD_TEST_WS_URL: config.backendWsUrl,
    FILFINANCE_LOAD_TEST_HLS_BASE_URL: config.hlsBaseUrl,
    FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS: config.allowedHlsHosts,
    FILFINANCE_EDGE_WS_URL: config.edgeWsUrl,
    FILFINANCE_EDGE_PUBLISH_URL: config.edgePublishUrl,
    FILFINANCE_COMBINED_LOAD_TEST_TIERS: String(clients),
    FILFINANCE_COMBINED_LOAD_TEST_DURATION_MS: String(durationMs),
  };
  const child = spawn(process.execPath, ["scripts/load_edge_ws_hls_combined.mjs"], {
    cwd: repoRoot,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const parsed = {
    hls_tier_report: null,
    edge_tier_report: null,
    hls_done_report: null,
    edge_done_report: null,
    child_lines: 0,
    stderr_lines: 0,
  };
  let reportsCompleteResolve = null;
  let reportsCompleteResolved = false;
  const reportsComplete = new Promise((resolve) => {
    reportsCompleteResolve = resolve;
  });
  const childDone = waitForChild(child);
  const resolveReportsCompleteIfReady = () => {
    if (!reportsCompleteResolved && parsed.hls_tier_report && parsed.edge_tier_report) {
      reportsCompleteResolved = true;
      reportsCompleteResolve();
    }
  };

  pipeChildLines(child.stdout, (line) => {
    parsed.child_lines += 1;
    recordCombinedOutputLine(line, parsed, { clients, label, stream: "stdout" });
    resolveReportsCompleteIfReady();
  });
  pipeChildLines(child.stderr, (line) => {
    parsed.stderr_lines += 1;
    emit(
      {
        stage: "prod_full_pipeline_load_combined_stderr",
        clients,
        label,
        line,
      },
      { console: false },
    );
  });

  const outcome = await Promise.race([
    childDone.then(({ code, signal }) => ({ kind: "closed", code, signal })),
    reportsComplete.then(async () => {
      await sleep(config.combinedReportGraceMs);
      const childClose = await terminateChild(child, childDone);
      return {
        kind: "terminated_after_reports",
        code:
          parsed.hls_tier_report?.ok === true && parsed.edge_tier_report?.ok === true ? 0 : 1,
        signal: childClose.signal ?? "SIGTERM",
        child_close: childClose,
      };
    }),
    sleep(durationMs + config.combinedMaxOverrunMs).then(async () => {
      const childClose = await terminateChild(child, childDone, "SIGKILL");
      return {
        kind: "timeout",
        code: 124,
        signal: childClose.signal ?? "SIGKILL",
        child_close: childClose,
      };
    }),
  ]);

  return {
    code: outcome.code,
    signal: outcome.signal,
    outcome,
    duration_ms: round(performance.now() - startedAtMs),
    parsed,
  };
}

function recordCombinedOutputLine(line, parsed, context) {
  const payload = parseJson(line);
  emit(
    {
      stage: "prod_full_pipeline_load_combined_output",
      ...context,
      payload: payload ?? line,
    },
    { console: false },
  );

  if (!payload || payload.stage !== "combined_child_output") {
    return;
  }

  const childPayload = payload.payload;
  if (!childPayload || typeof childPayload !== "object") {
    return;
  }

  if (childPayload.stage === "tier_report") {
    if (payload.child === "hls") {
      parsed.hls_tier_report = childPayload.report;
    } else if (payload.child === "edge") {
      parsed.edge_tier_report = childPayload.report;
    }
  }

  if (childPayload.stage === "load_test_done") {
    if (payload.child === "hls") {
      parsed.hls_done_report = childPayload.report;
    } else if (payload.child === "edge") {
      parsed.edge_done_report = childPayload.report;
    }
  }
}

async function startTestRunInjector(accessToken, clients, durationMs) {
  if (!accessToken) {
    return {
      stop: async () => ({ enabled: false, error: "missing access token" }),
    };
  }

  const wsUrl = deriveTraceWsUrl(config.backendWsUrl);
  const report = {
    enabled: true,
    ws_url: redactUrl(wsUrl),
    interval_ms: config.testRunIntervalMs,
    planned_duration_ms: durationMs,
    sent: 0,
    accepted: 0,
    errors: 0,
    lagged: 0,
    trace_events: 0,
    stage_counts: {},
    run_ids: [],
    open: false,
    close: null,
    error: null,
  };

  if (config.testRunIntervalMs <= 0) {
    report.enabled = false;
    report.error = "disabled by interval";
    return { stop: async () => report };
  }

  const socket = new WebSocket(wsUrl);
  let interval = null;
  let settled = false;
  const startPromise = new Promise((resolve) => {
    const timeout = setTimeout(() => {
      if (settled) {
        return;
      }
      settled = true;
      report.error = "test-run websocket open timeout";
      tryCloseWebSocket(socket);
      resolve();
    }, config.requestTimeoutMs);

    socket.addEventListener("open", () => {
      report.open = true;
      socket.send(JSON.stringify({ type: "auth", access_token: accessToken }));
      interval = setInterval(() => {
        if (socket.readyState !== WebSocket.OPEN) {
          report.errors += 1;
          return;
        }
        report.sent += 1;
        socket.send(JSON.stringify({ type: "trigger_test_run" }));
      }, config.testRunIntervalMs);
      if (!settled) {
        settled = true;
        clearTimeout(timeout);
        resolve();
      }
    });

    socket.addEventListener("message", (event) => {
      if (typeof event.data !== "string") {
        return;
      }
      const message = parseJson(event.data);
      if (!message) {
        report.errors += 1;
        return;
      }
      if (message.type === "test_run_accepted") {
        report.accepted += 1;
        if (message.payload?.run_id) {
          report.run_ids.push(message.payload.run_id);
        }
      } else if (message.type === "test_trace") {
        report.trace_events += 1;
        const stage = message.payload?.stage ?? "unknown";
        report.stage_counts[stage] = (report.stage_counts[stage] ?? 0) + 1;
      } else if (message.type === "lagged") {
        report.lagged += Number(message.payload?.skipped ?? 1);
      } else if (message.type === "error") {
        report.errors += 1;
        report.error = message.payload?.message ?? "test-run websocket error message";
      }
    });

    socket.addEventListener("close", (event) => {
      report.close = { code: event.code, reason: event.reason };
    });

    socket.addEventListener("error", () => {
      report.errors += 1;
      report.error = report.error || "test-run websocket error";
      if (!settled) {
        settled = true;
        clearTimeout(timeout);
        resolve();
      }
    });
  });

  await startPromise;

  return {
    stop: async () => {
      if (interval) {
        clearInterval(interval);
      }
      tryCloseWebSocket(socket);
      await sleep(250);
      emit({ stage: "prod_full_pipeline_load_test_run_report", clients, report });
      return report;
    },
  };
}

function startRuntimeProbeLoop(accessToken, label) {
  const reports = [];
  let stopped = false;
  const probe = async () => {
    if (stopped) {
      return;
    }
    reports.push(await collectRuntimeProbe(label, accessToken));
  };
  const interval = setInterval(() => {
    void probe();
  }, config.runtimeProbeIntervalMs);
  void probe();

  return {
    stop: async () => {
      stopped = true;
      clearInterval(interval);
      await sleep(100);
      return {
        count: reports.length,
        latest: reports[reports.length - 1] ?? null,
        errors: reports.reduce((count, report) => count + report.errors.length, 0),
      };
    },
  };
}

async function recordRuntimeProbe(label, accessToken) {
  const report = await collectRuntimeProbe(label, accessToken);
  emit({ stage: "prod_full_pipeline_load_runtime_probe", label, report });
  return report;
}

async function collectRuntimeProbe(label, accessToken) {
  const headers = accessToken ? { Authorization: `Bearer ${accessToken}` } : {};
  const endpoints = {
    health: "/health",
    admin_health: "/api/admin/health",
    translation: "/api/runtime/translation",
    tts: "/api/runtime/tts",
    hls: "/api/runtime/hls",
  };
  const entries = await Promise.all(
    Object.entries(endpoints).map(async ([name, path]) => {
      const startedAt = performance.now();
      try {
        const response = await fetchWithTimeout(new URL(path, `${config.apiBaseUrl}/`).toString(), {
          headers,
        });
        const body = await responseJsonOrText(response);
        return [
          name,
          {
            ok: response.ok,
            status: response.status,
            latency_ms: round(performance.now() - startedAt),
            body,
          },
        ];
      } catch (error) {
        return [
          name,
          {
            ok: false,
            status: null,
            latency_ms: round(performance.now() - startedAt),
            error: errorMessage(error),
          },
        ];
      }
    }),
  );
  const probes = Object.fromEntries(entries);
  return {
    label,
    checked_at: new Date().toISOString(),
    probes,
    errors: Object.entries(probes)
      .filter(([, probe]) => !probe.ok)
      .map(([name, probe]) => ({ name, status: probe.status, error: probe.error ?? null })),
  };
}

async function triggerCalendarDay(accessToken, { reasonSuffix, clients }) {
  const body = {
    force: config.calendarForce,
    reason: buildCalendarReason("day", reasonSuffix),
    ...(config.calendarLocalDay ? { localDay: config.calendarLocalDay } : {}),
  };
  const report = await postAdminJson("/api/admin/calendar-day/run", body, accessToken);
  calendarLastStableDayTriggered = calendarLastStableDayTriggered || reasonSuffix.startsWith("last_stable");
  emit({
    stage: "prod_full_pipeline_load_calendar_day_trigger",
    clients,
    reason_suffix: reasonSuffix,
    report,
  });
  return report;
}

async function triggerCalendarWeek(accessToken, { reasonSuffix, clients }) {
  const body = {
    force: config.calendarForce,
    reason: buildCalendarReason("week", reasonSuffix),
    ...(config.calendarReferenceDay ? { referenceDay: config.calendarReferenceDay } : {}),
  };
  const report = await postAdminJson("/api/admin/calendar-week/run", body, accessToken);
  emit({
    stage: "prod_full_pipeline_load_calendar_week_trigger",
    clients,
    reason_suffix: reasonSuffix,
    report,
  });
  return report;
}

async function postAdminJson(path, body, accessToken) {
  if (!accessToken) {
    return { ok: false, status: null, error: "missing access token" };
  }
  const startedAt = performance.now();
  try {
    const response = await fetchWithTimeout(new URL(path, `${config.apiBaseUrl}/`).toString(), {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
    return {
      ok: response.ok,
      status: response.status,
      latency_ms: round(performance.now() - startedAt),
      body: await responseJsonOrText(response),
    };
  } catch (error) {
    return {
      ok: false,
      status: null,
      latency_ms: round(performance.now() - startedAt),
      error: errorMessage(error),
    };
  }
}

async function recordSystemSnapshot(label) {
  if (!config.enableSystemSnapshots) {
    return null;
  }

  const snapshot = {
    label,
    checked_at: new Date().toISOString(),
    os: {
      hostname: safeCall(() => os.hostname(), null),
      platform: safeCall(() => os.platform(), null),
      release: safeCall(() => os.release(), null),
      arch: safeCall(() => os.arch(), null),
      cpus: safeCall(() => os.cpus().length, null),
      loadavg: safeCall(() => os.loadavg(), []),
      totalmem: safeCall(() => os.totalmem(), null),
      freemem: safeCall(() => os.freemem(), null),
      uptime: safeCall(() => os.uptime(), null),
    },
    commands: {},
  };

  const commands = [
    ["hostname", []],
    ["uname", ["-a"]],
    ["uptime", []],
    ["nproc", []],
    ["lscpu", []],
    ["free", ["-m"]],
    ["df", ["-h"]],
    ["docker", ["ps", "--format", "{{json .}}"]],
    ["docker", ["stats", "--no-stream", "--format", "{{json .}}"]],
    [
      "nvidia-smi",
      [
        "--query-gpu=name,driver_version,memory.total,utilization.gpu,utilization.memory",
        "--format=csv,noheader,nounits",
      ],
    ],
  ];

  for (const [command, commandArgs] of commands) {
    snapshot.commands[`${command} ${commandArgs.join(" ")}`.trim()] = await execFileReport(
      command,
      commandArgs,
    );
  }

  emit({ stage: "prod_full_pipeline_load_system_snapshot", label, snapshot }, { console: false });
  emit({
    stage: "prod_full_pipeline_load_system_snapshot_summary",
    label,
    hostname: snapshot.os.hostname,
    cpus: snapshot.os.cpus,
    loadavg: snapshot.os.loadavg,
    totalmem_gb: numberOrNull(snapshot.os.totalmem) === null
      ? null
      : round(snapshot.os.totalmem / 1024 / 1024 / 1024),
    freemem_gb: numberOrNull(snapshot.os.freemem) === null
      ? null
      : round(snapshot.os.freemem / 1024 / 1024 / 1024),
    has_nvidia_smi: snapshot.commands["nvidia-smi --query-gpu=name,driver_version,memory.total,utilization.gpu,utilization.memory --format=csv,noheader,nounits"].ok,
  });
  return snapshot;
}

function deriveTierMetrics(combined) {
  const hls = combined.parsed.hls_tier_report?.hls ?? null;
  const edge = combined.parsed.edge_tier_report ?? null;
  const ticket = hls?.ticket ?? null;
  const playlist = hls?.playlist ?? null;
  const segment = hls?.segment ?? null;
  const hlsRequests = sumNumbers(ticket?.requests, playlist?.requests, segment?.requests);
  const hlsFailures = sumNumbers(ticket?.failures, playlist?.failures, segment?.failures);
  const p95Values = [ticket?.latency_ms?.p95, playlist?.latency_ms?.p95, segment?.latency_ms?.p95]
    .filter((value) => typeof value === "number" && Number.isFinite(value));

  return {
    hls_requests: hlsRequests,
    hls_failures: hlsFailures,
    hls_error_rate: hlsRequests > 0 ? roundRatio(hlsFailures / hlsRequests) : null,
    hls_audio_delivery_p95_ms: p95Values.length > 0 ? round(Math.max(...p95Values)) : null,
    hls_total_bytes: hls?.total_bytes ?? null,
    hls_5xx_total: sumHttpStatusFamily(ticket, 5) + sumHttpStatusFamily(playlist, 5) + sumHttpStatusFamily(segment, 5),
    hls_ticket_requests: ticket?.requests ?? null,
    hls_ticket_failures: ticket?.failures ?? null,
    hls_ticket_retry_attempts: ticket?.retry_attempts ?? null,
    hls_ticket_retried_successes: ticket?.retried_successes ?? null,
    hls_ticket_p95_ms: ticket?.latency_ms?.p95 ?? null,
    hls_ticket_p99_ms: ticket?.latency_ms?.p99 ?? null,
    hls_playlist_requests: playlist?.requests ?? null,
    hls_playlist_failures: playlist?.failures ?? null,
    hls_playlist_retry_attempts: playlist?.retry_attempts ?? null,
    hls_playlist_retried_successes: playlist?.retried_successes ?? null,
    hls_playlist_p95_ms: playlist?.latency_ms?.p95 ?? null,
    hls_playlist_p99_ms: playlist?.latency_ms?.p99 ?? null,
    hls_segment_requests: segment?.requests ?? null,
    hls_segment_failures: segment?.failures ?? null,
    hls_segment_retry_attempts: segment?.retry_attempts ?? null,
    hls_segment_retried_successes: segment?.retried_successes ?? null,
    hls_segment_p95_ms: segment?.latency_ms?.p95 ?? null,
    hls_segment_p99_ms: segment?.latency_ms?.p99 ?? null,
    edge_latency_p95_ms: edge?.latency_ms?.p95 ?? null,
    edge_missed_deliveries: edge?.publish?.missed_deliveries ?? null,
    edge_connected: edge?.ws?.connected ?? null,
    edge_connection_failures: edge?.ws?.connection_failures ?? null,
    combined_exit_code: combined.code,
  };
}

function deriveGeneratorCost({
  elapsedMs,
  processStart,
  processEnd,
  memoryStart,
  memoryEnd,
  loadStart,
  loadEnd,
}) {
  const cpuCount = os.cpus().length || 1;
  const cpuMs =
    (processEnd.userCPUTime - processStart.userCPUTime) / 1000 +
    (processEnd.systemCPUTime - processStart.systemCPUTime) / 1000;
  const processCpuRatio = elapsedMs > 0 ? cpuMs / elapsedMs / cpuCount : 0;
  const loadRatio = loadEnd[0] / cpuCount;
  const freeMemoryRatio = os.freemem() / os.totalmem();
  const limited =
    loadRatio >= config.generatorLoadRatioLimit ||
    freeMemoryRatio <= config.generatorFreeMemoryRatioLimit;

  return {
    elapsed_ms: elapsedMs,
    cpu_count: cpuCount,
    total_memory_gb: round(os.totalmem() / 1024 / 1024 / 1024),
    process_cpu_ratio_estimate: roundRatio(processCpuRatio),
    loadavg_start: loadStart,
    loadavg_end: loadEnd,
    load_ratio_end: roundRatio(loadRatio),
    memory_start: memoryStart,
    memory_end: memoryEnd,
    free_memory_ratio_end: roundRatio(freeMemoryRatio),
    limited,
    limited_reason:
      loadRatio >= config.generatorLoadRatioLimit
        ? "loadavg"
        : freeMemoryRatio <= config.generatorFreeMemoryRatioLimit
          ? "memory"
          : null,
  };
}

async function finishSummary(branch, commit) {
  const capacity = summarizeCapacity();
  const summary = {
    run_id: config.runId,
    profile: config.profile,
    branch,
    commit,
    dry_run: config.dryRun,
    finished_at: new Date().toISOString(),
    stop_reason: stopReason,
    baseline_p95_ms: baselineP95Ms,
    last_stable_tier: lastStableTier,
    capacity,
    hls_p95_by_tier: summarizeHlsP95ByTier(),
    generator_cost: summarizeGeneratorCost(),
    tiers: tierResults.map(summarizeTier),
    output: config.dryRun ? null : { run_dir: runDir, events_jsonl: jsonlPath, summary: summaryPath },
  };
  emit({ stage: "prod_full_pipeline_load_done", summary });
  if (!config.dryRun) {
    await writeFileAtomic(summaryPath, JSON.stringify(summary, null, 2));
    outputStream?.end();
  }
}

function summarizeCapacity() {
  return {
    last_stable_tier: lastStableTier,
    error_threshold_rate: config.hlsErrorThreshold,
    error_threshold_tier: firstTierMatching((tier) => tier.evaluation?.error_threshold_crossed),
    latency_threshold_multiplier: config.hlsLatencyMultiplier,
    latency_threshold_tier: firstTierMatching((tier) => tier.evaluation?.latency_threshold_crossed),
  };
}

function summarizeHlsP95ByTier() {
  return Object.fromEntries(
    tierResults
      .filter((tier) => !tier.warmup)
      .map((tier) => [
        String(tier.clients),
        {
          label: tier.label,
          ticket_ms: tier.metrics.hls_ticket_p95_ms,
          playlist_ms: tier.metrics.hls_playlist_p95_ms,
          segment_ms: tier.metrics.hls_segment_p95_ms,
          audio_delivery_ms: tier.metrics.hls_audio_delivery_p95_ms,
        },
      ]),
  );
}

function summarizeGeneratorCost() {
  const latest = [...tierResults].reverse().find((tier) => !tier.warmup)?.generator ?? null;
  if (!latest) {
    return null;
  }

  return {
    cpu_count: latest.cpu_count,
    total_memory_gb: latest.total_memory_gb,
    process_cpu_ratio_estimate: latest.process_cpu_ratio_estimate,
    load_ratio_end: latest.load_ratio_end,
    free_memory_ratio_end: latest.free_memory_ratio_end,
    limited: latest.limited,
    limited_reason: latest.limited_reason,
  };
}

function firstTierMatching(predicate) {
  const tier = tierResults.find((result) => !result.warmup && predicate(result));
  return tier?.clients ?? null;
}

function summarizeTier(result) {
  return {
    label: result.label,
    clients: result.clients,
    duration_ms: result.duration_ms,
    warmup: result.warmup,
    capacity_status: result.capacity_status ?? null,
    metrics: result.metrics,
    generator: {
      cpu_count: result.generator.cpu_count,
      total_memory_gb: result.generator.total_memory_gb,
      load_ratio_end: result.generator.load_ratio_end,
      free_memory_ratio_end: result.generator.free_memory_ratio_end,
      limited: result.generator.limited,
      limited_reason: result.generator.limited_reason,
    },
    test_runs: result.test_runs
      ? {
          sent: result.test_runs.sent,
          accepted: result.test_runs.accepted,
          errors: result.test_runs.errors,
          lagged: result.test_runs.lagged,
          stage_counts: result.test_runs.stage_counts,
        }
      : null,
    runtime_probes: result.runtime_probes,
    combined: summarizeCombined(result.combined),
    evaluation: result.evaluation ?? null,
  };
}

function summarizeCombined(combined) {
  return {
    code: combined.code,
    signal: combined.signal,
    duration_ms: combined.duration_ms,
    parsed: {
      hls_has_tier_report: Boolean(combined.parsed.hls_tier_report),
      edge_has_tier_report: Boolean(combined.parsed.edge_tier_report),
      child_lines: combined.parsed.child_lines,
      stderr_lines: combined.parsed.stderr_lines,
    },
  };
}

function shouldTriggerCalendarDay(clients) {
  return config.enableCalendar && config.calendarDayTiers.includes(clients);
}

function shouldTriggerCalendarWeek(clients) {
  return (
    config.enableCalendar &&
    !calendarWeekTriggered &&
    config.calendarWeekAfterTier !== null &&
    clients === config.calendarWeekAfterTier
  );
}

function buildCalendarReason(kind, suffix) {
  return `load_test_${config.profile}_${config.runId}_${kind}_${suffix}`.slice(0, 240);
}

function validateConfig() {
  if (!config.dryRun && config.confirm !== "prod-real") {
    throw new Error(
      "Refusing to run against production. Set FILFINANCE_PROD_LOAD_CONFIRM=prod-real or use --dry-run.",
    );
  }
  if (!config.hlsBaseUrl) {
    throw new Error(
      "FILFINANCE_LOAD_TEST_HLS_BASE_URL is required; Cloudflare live.filfinance.fr is not a safe default.",
    );
  }
  assertAllowedHlsLoadTestTarget(config.hlsBaseUrl);
  if (!config.dryRun && typeof fetch !== "function") {
    throw new Error("global fetch is unavailable; run this script with Node 18+");
  }
  if (!config.dryRun && typeof WebSocket !== "function") {
    throw new Error("global WebSocket is unavailable; run this script with the repo Node runtime");
  }
  if (!config.tiers.includes(config.baselineTier)) {
    throw new Error(`FILFINANCE_PROD_LOAD_TIERS must include baseline tier ${config.baselineTier}`);
  }
  if (config.tiers.length === 0) {
    throw new Error("FILFINANCE_PROD_LOAD_TIERS cannot be empty");
  }
  if (!process.env.FILFINANCE_EDGE_PUBLISH_SECRET && !config.dryRun) {
    throw new Error("FILFINANCE_EDGE_PUBLISH_SECRET is required for real Edge load tests");
  }
}

function assertAllowedHlsLoadTestTarget(baseUrl) {
  const parsed = new URL(baseUrl);
  const hostname = parsed.hostname.toLowerCase();
  const allowedMediaHosts = csvSet(config.allowedHlsHosts);
  if (isBunnyCdnHost(hostname) || allowedMediaHosts.has(hostname)) {
    return;
  }

  const isFilfinanceHost = hostname === "filfinance.fr" || hostname.endsWith(".filfinance.fr");
  const isLiveFilfinanceHost =
    hostname === "live.filfinance.fr" ||
    (hostname.startsWith("live-") && hostname.endsWith(".filfinance.fr"));
  const blockedSuffixes = ["workers.dev", "cloudflarestream.com", "videodelivery.net"];
  const matchedPattern = isFilfinanceHost
    ? "filfinance.fr"
    : blockedSuffixes.find((pattern) => hostname === pattern || hostname.endsWith(`.${pattern}`));
  if (!matchedPattern) {
    return;
  }

  if (
    envBool("FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST", false) &&
    (!isFilfinanceHost || isLiveFilfinanceHost)
  ) {
    return;
  }

  throw new Error(
    [
      "Cloudflare media load tests are disabled by default.",
      `Blocked HLS base URL: ${redactUrl(baseUrl)}`,
      "Set FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS for Bunny/media hosts.",
      "Only use FILFINANCE_ALLOW_CLOUDFLARE_MEDIA_LOAD_TEST=true for an explicit Cloudflare test.",
    ].join(" "),
  );
}

async function resolveAccessToken() {
  const directToken = process.env.FILFINANCE_LOAD_TEST_ACCESS_TOKEN?.trim();
  if (directToken) {
    return directToken;
  }

  throw new Error(
    [
      "Production load test requires auth.",
      "Set FILFINANCE_LOAD_TEST_ACCESS_TOKEN to a Better Auth JWT.",
    ].join(" "),
  );
}

async function fetchWithTimeout(url, options = {}) {
  if (typeof fetch !== "function") {
    throw new Error("global fetch is unavailable; run this script with Node 18+");
  }
  if (typeof AbortController !== "function") {
    return await fetch(url, options);
  }
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), config.requestTimeoutMs);
  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } finally {
    clearTimeout(timeout);
  }
}

async function responseJsonOrText(response) {
  const text = await response.text();
  if (!text) {
    return null;
  }
  try {
    return JSON.parse(text);
  } catch {
    return text.slice(0, 2048);
  }
}

function pipeChildLines(stream, onLine) {
  let buffer = "";
  stream.setEncoding("utf8");
  stream.on("data", (chunk) => {
    buffer += chunk;
    let newlineIndex = buffer.indexOf("\n");
    while (newlineIndex >= 0) {
      onLine(buffer.slice(0, newlineIndex).trimEnd());
      buffer = buffer.slice(newlineIndex + 1);
      newlineIndex = buffer.indexOf("\n");
    }
  });
  stream.on("end", () => {
    if (buffer.trim()) {
      onLine(buffer.trimEnd());
    }
  });
}

function waitForChild(child) {
  return new Promise((resolve) => {
    child.on("close", (code, signal) => resolve({ code, signal }));
  });
}

async function terminateChild(child, childDone, signal = "SIGTERM") {
  if (child.exitCode !== null || child.signalCode !== null) {
    return await childDone;
  }
  child.kill(signal);
  return await Promise.race([
    childDone,
    sleep(5_000).then(() => {
      if (child.exitCode === null && child.signalCode === null) {
        child.kill("SIGKILL");
      }
      return childDone;
    }),
  ]);
}

async function execFileReport(command, commandArgs) {
  return new Promise((resolve) => {
    execFile(
      command,
      commandArgs,
      { cwd: repoRoot, timeout: 10_000, maxBuffer: 1024 * 1024 },
      (error, stdout, stderr) => {
        resolve({
          ok: !error,
          code: error?.code ?? 0,
          signal: error?.signal ?? null,
          stdout: String(stdout || "").slice(0, 40_000),
          stderr: String(stderr || "").slice(0, 10_000),
          error: error ? errorMessage(error) : null,
        });
      },
    );
  });
}

async function currentGitBranch() {
  const report = await execFileReport("git", ["rev-parse", "--abbrev-ref", "HEAD"]);
  return report.ok ? report.stdout.trim() : null;
}

async function currentGitCommit() {
  const report = await execFileReport("git", ["rev-parse", "HEAD"]);
  return report.ok ? report.stdout.trim() : null;
}

async function writeFileAtomic(path, contents) {
  const { writeFile, rename } = await import("fs/promises");
  const tmpPath = `${path}.tmp`;
  await writeFile(tmpPath, contents);
  await rename(tmpPath, path);
}

function emit(event, options = {}) {
  const payload = {
    timestamp: new Date().toISOString(),
    ...event,
  };
  const serialized = JSON.stringify(payload);
  if (options.console !== false) {
    console.log(serialized);
  }
  if (outputStream) {
    outputStream.write(`${serialized}\n`);
  }
}

function defaultTiersForProfile(profile) {
  const base = "50,100,200,500,750,1000,1500,2000";
  return profile === "prod-perf" ? `${base},2500,3000` : base;
}

function deriveTraceWsUrl(wsUrl) {
  const parsed = new URL(wsUrl);
  parsed.pathname = "/ws/test-runs";
  parsed.search = "";
  parsed.hash = "";
  return parsed.toString();
}

function tryCloseWebSocket(socket) {
  try {
    if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
      socket.close();
    }
  } catch {
    // Best-effort cleanup only.
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function sumNumbers(...values) {
  return values.reduce(
    (total, value) => total + (typeof value === "number" && Number.isFinite(value) ? value : 0),
    0,
  );
}

function sumHttpStatusFamily(report, family) {
  const statusCodes = report?.status_codes;
  if (!statusCodes || typeof statusCodes !== "object") {
    return 0;
  }
  const prefix = String(family);
  return Object.entries(statusCodes).reduce((total, [status, count]) => {
    if (!String(status).startsWith(prefix)) {
      return total;
    }
    return total + (typeof count === "number" && Number.isFinite(count) ? count : 0);
  }, 0);
}

function safeCall(callback, fallback) {
  try {
    return callback();
  } catch {
    return fallback;
  }
}

function numberOrNull(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function round(value) {
  return Number(value.toFixed(2));
}

function roundRatio(value) {
  return Number(value.toFixed(6));
}

function parseJson(raw) {
  if (typeof raw !== "string") {
    return null;
  }
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function redactUrl(value) {
  const parsed = new URL(value);
  parsed.username = "";
  parsed.password = "";
  parsed.search = "";
  return parsed.toString();
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function timestampSlug() {
  return new Date().toISOString().replace(/[-:]/gu, "").replace(/\.\d{3}Z$/u, "Z");
}

function envString(name, defaultValue) {
  return process.env[name]?.trim() || defaultValue;
}

function envBool(name, defaultValue) {
  const value = process.env[name]?.trim().toLowerCase();
  if (!value) {
    return defaultValue;
  }
  return ["1", "true", "yes", "on"].includes(value);
}

function envInt(name, defaultValue) {
  const value = Number.parseInt(process.env[name] ?? "", 10);
  return Number.isFinite(value) ? value : defaultValue;
}

function envOptionalInt(name, defaultValue) {
  if (!process.env[name]?.trim()) {
    return defaultValue;
  }
  const value = Number.parseInt(process.env[name] ?? "", 10);
  return Number.isFinite(value) ? value : defaultValue;
}

function envFloat(name, defaultValue) {
  const value = Number.parseFloat(process.env[name] ?? "");
  return Number.isFinite(value) ? value : defaultValue;
}

function envIntList(name, defaultValue) {
  const raw = process.env[name]?.trim() || defaultValue;
  return raw
    .split(",")
    .map((value) => Number.parseInt(value.trim(), 10))
    .filter((value) => Number.isFinite(value) && value > 0);
}

function normalizeOptionalBaseUrl(value) {
  const trimmed = String(value || "").trim();
  if (!trimmed) {
    return "";
  }

  const parsed = new URL(trimmed);
  parsed.pathname = parsed.pathname.replace(/\/+$/u, "");
  parsed.search = "";
  parsed.hash = "";
  return parsed.toString().replace(/\/$/u, "");
}

function isBunnyCdnHost(hostname) {
  return hostname === "b-cdn.net" || hostname.endsWith(".b-cdn.net");
}

function csvSet(raw) {
  return new Set(
    String(raw || "")
      .split(",")
      .map((value) => value.trim().toLowerCase())
      .filter(Boolean),
  );
}

function loadEnvFile(path) {
  if (!path || !existsSync(path)) {
    return;
  }
  const contents = readFileSync(path, "utf8");
  for (const line of contents.split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }
    const separator = trimmed.indexOf("=");
    if (separator <= 0) {
      continue;
    }
    const key = trimmed.slice(0, separator).trim();
    let value = trimmed.slice(separator + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (!process.env[key]) {
      process.env[key] = value;
    }
  }
}

function printHelp() {
  console.log(`Usage:
  FILFINANCE_PROD_LOAD_CONFIRM=prod-real node scripts/load_prod_full_pipeline.mjs
  node scripts/load_prod_full_pipeline.mjs --dry-run

Required for real runs:
  FILFINANCE_PROD_LOAD_CONFIRM=prod-real
  FILFINANCE_EDGE_PUBLISH_SECRET
  FILFINANCE_LOAD_TEST_HLS_BASE_URL=https://media.filfinance.fr
  FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=media.filfinance.fr
  FILFINANCE_LOAD_TEST_ACCESS_TOKEN

Key variables:
  FILFINANCE_PROD_LOAD_PROFILE=main|prod-perf
  FILFINANCE_PROD_LOAD_TIERS=50,100,200,500,750,1000,1500,2000
  FILFINANCE_PROD_LOAD_DURATION_MS=120000
  FILFINANCE_LOAD_TEST_HLS_BASE_URL=
  FILFINANCE_MEDIA_LOAD_TEST_ALLOWED_HOSTS=
  FILFINANCE_PROD_LOAD_TEST_RUN_INTERVAL_MS=15000
  FILFINANCE_PROD_LOAD_CALENDAR_DAY_TIERS=100,500
  FILFINANCE_PROD_LOAD_CALENDAR_WEEK_AFTER_TIER=500
  FILFINANCE_PROD_LOAD_OUTPUT_DIR=/tmp/filfinance-load-tests
`);
}
