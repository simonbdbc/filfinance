const fs = require("node:fs");
const path = require("node:path");

const projectRoot = path.join(__dirname, "..");
const audioPlaybackPath = path.join(
  projectRoot,
  "src",
  "features",
  "audio",
  "hooks",
  "useAudioPlayback.ts",
);
const hlsPlayerPath = path.join(
  projectRoot,
  "src",
  "features",
  "audio",
  "hooks",
  "useHlsAudioPlayer.ts",
);
const source = fs.readFileSync(audioPlaybackPath, "utf8");
const hlsSource = fs.readFileSync(hlsPlayerPath, "utf8");

function assertIncludes(expected, label) {
  if (!source.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

const pollFlag = "const shouldPollReplayTodayMetadata =";
const intervalCall = "window.setInterval(";
const idleGuard = "if (!shouldPollReplayTodayMetadata) {";

assertIncludes(pollFlag, "Replay metadata polling guard");
assertIncludes(idleGuard, "Replay metadata idle guard");
assertIncludes("hlsPlaybackMode === \"replay_today\"", "Replay-only polling mode");
assertIncludes("isHlsPlaybackActive", "Active HLS playback guard");
assertIncludes(intervalCall, "Replay metadata interval");

const idleGuardIndex = source.indexOf(idleGuard);
const intervalIndex = source.indexOf(intervalCall);

if (idleGuardIndex < 0 || intervalIndex < 0 || intervalIndex < idleGuardIndex) {
  throw new Error("Replay metadata interval must be created after the idle guard");
}

for (const expected of [
  "function hlsBoundedLoadPolicy()",
  "manifestLoadingMaxRetry: 0",
  "maxNumRetry: 1",
  "retryDelayMs: HLS_RETRY_DELAY_MS",
  "maxRetryDelayMs: HLS_MAX_RETRY_DELAY_MS",
]) {
  if (!hlsSource.includes(expected)) {
    throw new Error(`HLS retry/backoff contract: missing ${expected}`);
  }
}

console.log("[audio-idle-polling] replay metadata polling and HLS retries are bounded");
