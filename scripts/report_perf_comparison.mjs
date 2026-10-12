#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";

const args = parseArgs(process.argv.slice(2));

if (args.help || !args.dev || !args.prod) {
  printHelp();
  process.exit(args.help ? 0 : 1);
}

const outputDir = resolve(args.outDir || `perf-comparison-${timestampSlug()}`);
mkdirSync(outputDir, { recursive: true });

const devRun = normalizeRun(readRun(args.dev), "current dev");
const prodRun = normalizeRun(readRun(args.prod), "improved prod-perf");
const comparison = buildComparison(devRun, prodRun);
const charts = writeCharts(outputDir, devRun, prodRun);
const markdown = renderMarkdown({ devRun, prodRun, comparison, charts });
const markdownPath = resolve(outputDir, "comparison.md");
const dataPath = resolve(outputDir, "comparison-data.json");

writeFileSync(markdownPath, markdown);
writeFileSync(dataPath, JSON.stringify({ devRun, prodRun, comparison }, null, 2));

console.log(
  JSON.stringify({
    stage: "perf_comparison_report_written",
    output_dir: outputDir,
    markdown: markdownPath,
    data: dataPath,
    charts,
  }),
);

function readRun(path) {
  const resolved = resolve(path);
  if (!existsSync(resolved)) {
    throw new Error(`Run file not found: ${resolved}`);
  }

  const raw = readFileSync(resolved, "utf8").trim();
  if (!raw) {
    throw new Error(`Run file is empty: ${resolved}`);
  }

  if (looksLikeJsonl(raw)) {
    return readJsonlRun(raw, resolved);
  }

  const payload = JSON.parse(raw);
  return payload.report ?? payload.summary ?? payload;
}

function readJsonlRun(raw, path) {
  const events = raw
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  const done = [...events].reverse().find((event) =>
    event.stage === "load_test_done" ||
    event.stage === "prod_full_pipeline_load_done"
  );
  if (done?.report) {
    return done.report;
  }
  if (done?.summary) {
    return done.summary;
  }

  const tiers = events
    .filter((event) => event.stage === "tier_report" && event.report)
    .map((event) => event.report);
  if (tiers.length > 0) {
    return { run_id: basename(path), tiers };
  }

  throw new Error(`No load summary found in JSONL: ${path}`);
}

function normalizeRun(summary, fallbackName) {
  const tiers = (summary.tiers ?? [])
    .map(normalizeTier)
    .filter((tier) => Number.isFinite(tier.clients))
    .sort((left, right) => left.clients - right.clients);
  const baselineP95 = pickBaselineP95(summary, tiers);

  return {
    name: summary.profile ?? summary.run_id ?? fallbackName,
    run_id: summary.run_id ?? fallbackName,
    branch: summary.branch ?? null,
    commit: summary.commit ?? null,
    finished_at: summary.finished_at ?? null,
    baseline_p95_ms: baselineP95,
    last_stable_tier: summary.last_stable_tier ?? lastStableTier(tiers),
    error_threshold_tier:
      summary.capacity?.error_threshold_tier ?? firstThresholdTier(tiers, "error", baselineP95),
    latency_threshold_tier:
      summary.capacity?.latency_threshold_tier ?? firstThresholdTier(tiers, "latency", baselineP95),
    p95_by_tier: hlsP95ByTier(tiers),
    resources: summary.generator_cost ?? latestGenerator(tiers),
    tiers,
  };
}

function normalizeTier(tier) {
  if (tier.metrics) {
    return {
      label: tier.label ?? null,
      clients: Number(tier.clients),
      ok: tier.combined?.code === 0 || tier.capacity_status === "stable" || tier.evaluation?.threshold_crossed === false,
      capacity_status: tier.capacity_status ?? null,
      metrics: tier.metrics,
      generator: tier.generator ?? null,
      runtime_probes: tier.runtime_probes ?? null,
    };
  }

  const hls = tier.hls ?? null;
  const metrics = {
    hls_requests: sumNumbers(
      hls?.ticket?.requests,
      hls?.playlist?.requests,
      hls?.segment?.requests,
    ),
    hls_failures: sumNumbers(
      hls?.ticket?.failures,
      hls?.playlist?.failures,
      hls?.segment?.failures,
    ),
    hls_ticket_failures: hls?.ticket?.failures ?? null,
    hls_playlist_failures: hls?.playlist?.failures ?? null,
    hls_segment_failures: hls?.segment?.failures ?? null,
    hls_total_bytes: hls?.total_bytes ?? null,
    hls_ticket_p95_ms: hls?.ticket?.latency_ms?.p95 ?? null,
    hls_playlist_p95_ms: hls?.playlist?.latency_ms?.p95 ?? null,
    hls_segment_p95_ms: hls?.segment?.latency_ms?.p95 ?? null,
  };
  metrics.hls_error_rate =
    metrics.hls_requests > 0 ? roundRatio(metrics.hls_failures / metrics.hls_requests) : null;
  metrics.hls_audio_delivery_p95_ms = maxNumber(
    metrics.hls_ticket_p95_ms,
    metrics.hls_playlist_p95_ms,
    metrics.hls_segment_p95_ms,
  );

  return {
    label: tier.label ?? null,
    clients: Number(tier.clientsRequested ?? tier.clients),
    ok: tier.ok === true,
    capacity_status: tier.ok === true ? "stable" : "failed",
    metrics,
    generator: null,
    runtime_probes: null,
  };
}

function buildComparison(devRun, prodRun) {
  const tiers = [500, 1000];
  const p95 = Object.fromEntries(
    tiers.map((tier) => {
      const dev = devRun.p95_by_tier[String(tier)]?.audio_delivery_ms ?? null;
      const prod = prodRun.p95_by_tier[String(tier)]?.audio_delivery_ms ?? null;
      return [
        tier,
        {
          dev_ms: dev,
          prod_perf_ms: prod,
          delta_ms: numberOrNull(prod) !== null && numberOrNull(dev) !== null ? round(prod - dev) : null,
          delta_pct: numberOrNull(prod) !== null && numberOrNull(dev) ? roundRatio((prod - dev) / dev) : null,
        },
      ];
    }),
  );

  return {
    last_stable_tier_delta: numericDelta(prodRun.last_stable_tier, devRun.last_stable_tier),
    normalized_stable_per_vcpu: {
      dev: normalizedStableTier(devRun, "cpu"),
      prod_perf: normalizedStableTier(prodRun, "cpu"),
    },
    normalized_stable_per_gb_ram: {
      dev: normalizedStableTier(devRun, "ram"),
      prod_perf: normalizedStableTier(prodRun, "ram"),
    },
    p95,
  };
}

function renderMarkdown({ devRun, prodRun, comparison, charts }) {
  return `# Comparaison Performance HLS

## Synthese

| Run | Branche | Dernier palier stable | Seuil 10% erreurs | Seuil +20% p95 |
| --- | --- | ---: | ---: | ---: |
| ${devRun.name} | ${devRun.branch ?? "-"} | ${cell(devRun.last_stable_tier)} | ${cell(devRun.error_threshold_tier)} | ${cell(devRun.latency_threshold_tier)} |
| ${prodRun.name} | ${prodRun.branch ?? "-"} | ${cell(prodRun.last_stable_tier)} | ${cell(prodRun.error_threshold_tier)} | ${cell(prodRun.latency_threshold_tier)} |

Gain palier stable: ${cell(comparison.last_stable_tier_delta)} clients.
Gain normalise vCPU: ${cell(comparison.normalized_stable_per_vcpu.prod_perf)} vs ${cell(comparison.normalized_stable_per_vcpu.dev)}.
Gain normalise RAM: ${cell(comparison.normalized_stable_per_gb_ram.prod_perf)} vs ${cell(comparison.normalized_stable_per_gb_ram.dev)}.

## P95 HLS

| Palier | Dev p95 audio | Prod-perf p95 audio | Delta |
| ---: | ---: | ---: | ---: |
| 500 | ${ms(comparison.p95[500].dev_ms)} | ${ms(comparison.p95[500].prod_perf_ms)} | ${ms(comparison.p95[500].delta_ms)} |
| 1000 | ${ms(comparison.p95[1000].dev_ms)} | ${ms(comparison.p95[1000].prod_perf_ms)} | ${ms(comparison.p95[1000].delta_ms)} |

## Graphiques

- [Clients vs p95 HLS](${basename(charts.p95)})
- [Clients vs taux d'erreur](${basename(charts.errorRate)})
- [Clients vs CPU/RAM](${basename(charts.resources)})
- [Breakdown HLS](${basename(charts.breakdown)})

## Donnees

Le fichier machine-readable \`comparison-data.json\` contient les paliers normalises, les latences ticket/playlist/segment, les ressources generateur et les metadonnees de runs.
`;
}

function writeCharts(outputDir, devRun, prodRun) {
  const charts = {
    p95: resolve(outputDir, "clients-vs-p95-hls.svg"),
    errorRate: resolve(outputDir, "clients-vs-error-rate.svg"),
    resources: resolve(outputDir, "clients-vs-cpu-ram.svg"),
    breakdown: resolve(outputDir, "breakdown-hls.svg"),
  };

  writeFileSync(
    charts.p95,
    lineChart("Clients vs p95 HLS", [
      seriesFromRun(devRun, "Dev", (tier) => tier.metrics.hls_audio_delivery_p95_ms),
      seriesFromRun(prodRun, "Prod-perf", (tier) => tier.metrics.hls_audio_delivery_p95_ms),
    ], "ms"),
  );
  writeFileSync(
    charts.errorRate,
    lineChart("Clients vs taux d'erreur", [
      seriesFromRun(devRun, "Dev", (tier) => percent(tier.metrics.hls_error_rate)),
      seriesFromRun(prodRun, "Prod-perf", (tier) => percent(tier.metrics.hls_error_rate)),
    ], "%"),
  );
  writeFileSync(
    charts.resources,
    lineChart("Clients vs CPU/RAM generateur", [
      seriesFromRun(prodRun, "Load ratio", (tier) => percent(tier.generator?.load_ratio_end)),
      seriesFromRun(prodRun, "Free RAM", (tier) => percent(tier.generator?.free_memory_ratio_end)),
    ], "%"),
  );
  writeFileSync(
    charts.breakdown,
    lineChart("Breakdown HLS prod-perf", [
      seriesFromRun(prodRun, "Ticket", (tier) => tier.metrics.hls_ticket_p95_ms),
      seriesFromRun(prodRun, "Playlist", (tier) => tier.metrics.hls_playlist_p95_ms),
      seriesFromRun(prodRun, "Segment", (tier) => tier.metrics.hls_segment_p95_ms),
    ], "ms"),
  );

  return charts;
}

function lineChart(title, series, unit) {
  const width = 920;
  const height = 420;
  const pad = 56;
  const points = series.flatMap((item) => item.points);
  const maxX = Math.max(1, ...points.map((point) => point.x));
  const maxY = Math.max(1, ...points.map((point) => point.y));
  const colors = ["#2563eb", "#dc2626", "#16a34a", "#9333ea"];
  const x = (value) => pad + (value / maxX) * (width - pad * 2);
  const y = (value) => height - pad - (value / maxY) * (height - pad * 2);

  const paths = series
    .map((item, index) => {
      const d = item.points
        .map((point, pointIndex) => `${pointIndex === 0 ? "M" : "L"} ${x(point.x)} ${y(point.y)}`)
        .join(" ");
      const color = colors[index % colors.length];
      const dots = item.points
        .map((point) => `<circle cx="${x(point.x)}" cy="${y(point.y)}" r="4" fill="${color}" />`)
        .join("");
      return `<path d="${d}" fill="none" stroke="${color}" stroke-width="2.5" />${dots}`;
    })
    .join("");
  const legend = series
    .map((item, index) => {
      const color = colors[index % colors.length];
      const yPos = 28 + index * 20;
      return `<rect x="${width - 180}" y="${yPos - 10}" width="12" height="12" fill="${color}" /><text x="${width - 162}" y="${yPos}" font-size="13">${escapeXml(item.name)}</text>`;
    })
    .join("");

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
  <rect width="100%" height="100%" fill="#ffffff" />
  <text x="${pad}" y="30" font-size="20" font-family="Arial, sans-serif" font-weight="700">${escapeXml(title)}</text>
  <line x1="${pad}" y1="${height - pad}" x2="${width - pad}" y2="${height - pad}" stroke="#111827" />
  <line x1="${pad}" y1="${pad}" x2="${pad}" y2="${height - pad}" stroke="#111827" />
  <text x="${width / 2 - 35}" y="${height - 14}" font-size="13" font-family="Arial, sans-serif">clients</text>
  <text x="14" y="${pad - 14}" font-size="13" font-family="Arial, sans-serif">${escapeXml(unit)}</text>
  <text x="${pad}" y="${height - pad + 20}" font-size="12" font-family="Arial, sans-serif">0</text>
  <text x="${width - pad - 22}" y="${height - pad + 20}" font-size="12" font-family="Arial, sans-serif">${maxX}</text>
  <text x="12" y="${y(maxY) + 4}" font-size="12" font-family="Arial, sans-serif">${round(maxY)}</text>
  ${paths}
  ${legend}
</svg>
`;
}

function seriesFromRun(run, name, getter) {
  return {
    name,
    points: run.tiers
      .map((tier) => ({ x: tier.clients, y: numberOrNull(getter(tier)) }))
      .filter((point) => point.y !== null),
  };
}

function hlsP95ByTier(tiers) {
  return Object.fromEntries(
    tiers.map((tier) => [
      String(tier.clients),
      {
        ticket_ms: tier.metrics.hls_ticket_p95_ms ?? null,
        playlist_ms: tier.metrics.hls_playlist_p95_ms ?? null,
        segment_ms: tier.metrics.hls_segment_p95_ms ?? null,
        audio_delivery_ms: tier.metrics.hls_audio_delivery_p95_ms ?? null,
      },
    ]),
  );
}

function firstThresholdTier(tiers, kind, baselineP95) {
  const tier = tiers.find((item) => {
    if (kind === "error") {
      return numberOrNull(item.metrics.hls_error_rate) !== null && item.metrics.hls_error_rate >= 0.1;
    }
    return (
      numberOrNull(baselineP95) !== null &&
      numberOrNull(item.metrics.hls_audio_delivery_p95_ms) !== null &&
      item.metrics.hls_audio_delivery_p95_ms >= baselineP95 * 1.2
    );
  });
  return tier?.clients ?? null;
}

function lastStableTier(tiers) {
  const stable = tiers.filter((tier) =>
    tier.ok &&
    (tier.metrics.hls_ticket_failures ?? 0) === 0 &&
    (tier.metrics.hls_playlist_failures ?? 0) === 0 &&
    (tier.metrics.hls_segment_failures ?? 0) === 0
  );
  return stable.at(-1)?.clients ?? null;
}

function pickBaselineP95(summary, tiers) {
  if (numberOrNull(summary.baseline_p95_ms) !== null) {
    return summary.baseline_p95_ms;
  }
  return (
    tiers.find((tier) => tier.clients === 100)?.metrics.hls_audio_delivery_p95_ms ??
    tiers[0]?.metrics.hls_audio_delivery_p95_ms ??
    null
  );
}

function latestGenerator(tiers) {
  return [...tiers].reverse().find((tier) => tier.generator)?.generator ?? null;
}

function normalizedStableTier(run, denominator) {
  const stable = numberOrNull(run.last_stable_tier);
  if (stable === null) {
    return null;
  }
  if (denominator === "cpu") {
    const cpu = numberOrNull(run.resources?.cpu_count);
    return cpu ? round(stable / cpu) : null;
  }
  const ram = numberOrNull(run.resources?.total_memory_gb);
  return ram ? round(stable / ram) : null;
}

function parseArgs(argv) {
  const parsed = { dev: "", prod: "", outDir: "", help: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
    } else if (arg === "--dev") {
      parsed.dev = argv[++index] ?? "";
    } else if (arg === "--prod") {
      parsed.prod = argv[++index] ?? "";
    } else if (arg === "--out-dir") {
      parsed.outDir = argv[++index] ?? "";
    }
  }
  return parsed;
}

function looksLikeJsonl(raw) {
  const firstNewline = raw.indexOf("\n");
  if (firstNewline < 0) {
    return false;
  }
  const firstLine = raw.slice(0, firstNewline).trim();
  return firstLine.startsWith("{") && raw.slice(firstNewline + 1).trimStart().startsWith("{");
}

function sumNumbers(...values) {
  return values.reduce(
    (total, value) => total + (typeof value === "number" && Number.isFinite(value) ? value : 0),
    0,
  );
}

function maxNumber(...values) {
  const numbers = values.filter((value) => typeof value === "number" && Number.isFinite(value));
  return numbers.length > 0 ? Math.max(...numbers) : null;
}

function numericDelta(left, right) {
  return numberOrNull(left) !== null && numberOrNull(right) !== null ? left - right : null;
}

function numberOrNull(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function percent(value) {
  return numberOrNull(value) === null ? null : round(value * 100);
}

function cell(value) {
  return value === null || value === undefined ? "-" : String(value);
}

function ms(value) {
  return value === null || value === undefined ? "-" : `${round(value)} ms`;
}

function round(value) {
  return Number(value.toFixed(2));
}

function roundRatio(value) {
  return Number(value.toFixed(6));
}

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function timestampSlug() {
  return new Date().toISOString().replace(/[-:]/gu, "").replace(/\.\d{3}Z$/u, "Z");
}

function printHelp() {
  console.log(`Usage:
  node scripts/report_perf_comparison.mjs --dev DEV_SUMMARY_OR_JSONL --prod PROD_SUMMARY --out-dir perf-report

Inputs:
  --dev   summary.json or JSONL from scripts/load_backend_ws_hls.mjs
  --prod  summary.json from scripts/load_prod_full_pipeline.mjs
`);
}
