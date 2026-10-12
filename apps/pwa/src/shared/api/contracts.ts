export type ConnectionState =
  | "idle"
  | "waiting"
  | "connecting"
  | "open"
  | "closed"
  | "error";

export type TranslatedNewsItem = {
  id: number;
  news_event_id: number;
  language: string;
  status: string;
  source: string;
  event_kind: string;
  source_title: string;
  source_body: string | null;
  translated_title: string;
  translated_body: string | null;
  narration_text: string;
  translation_mode: string;
  error_message: string | null;
  news_level: string | null;
  test_run_id?: string | null;
  url?: string | null;
  timestamp?: string | null;
  created_at: string;
  updated_at: string;
};

export type SourceStatus = {
  name: string;
  enabled: boolean;
  configured_enabled?: boolean | null;
  url: string;
  poll_interval_ms: number;
  request_timeout_ms: number;
  status?: string | null;
  message?: string | null;
  updated_at?: string | null;
  service_accessible?: boolean | null;
  main_feed_available?: boolean | null;
  stale?: boolean | null;
};

export type PublicSourceStatus = {
  source: string;
  enabled: boolean;
  status: "active" | "disabled" | "blocked" | "degraded" | "stale";
  updated_at: string;
};

export type PublicCalendarDaySummaryStatus = {
  local_day: string;
  status:
    | "missing"
    | "pending"
    | "summarized"
    | "audio_generating"
    | "audio_ready"
    | "error"
    | string;
  audio_available: boolean;
  updated_at: string;
};

export type EdgeLiveAudioStatus =
  | "idle"
  | "buffering"
  | "playing"
  | "delayed"
  | "error";

export type EdgeLiveNoticeSeverity = "info" | "warning" | "error";

export type EdgeNewsCard = {
  news_event_id: number;
  translation_id: number | null;
  source: string;
  event_kind: string;
  source_title: string;
  translated_title: string | null;
  news_level: string | null;
  url: string | null;
  timestamp: string;
};

export type EdgeNowPlaying = {
  stream_id: string | null;
  translation_id: number | null;
  news_event_id: number | null;
  title: string | null;
  news_level: string | null;
  started_at: string;
  timestamp: string | null;
};

export type EdgeAudioState = {
  status: EdgeLiveAudioStatus;
  replay_today_playlist_url: string | null;
  detail: string | null;
  updated_at: string;
};

export type EdgeAudioReady = {
  translation_id: number;
  news_event_id?: number | null;
  timestamp?: string | null;
  stream_id: string;
  playlist_url: string;
  published_at: string;
  target_duration_secs: number;
};

export type EdgeAudioFailed = {
  translation_id: number | null;
  stream_id: string | null;
  error_message: string;
  failed_at: string;
};

export type EdgeAudioFinalized = {
  translation_id: number;
  stream_id: string;
  playlist_url: string;
  finalized_at: string;
};

export type EdgeCalendarAlertReady = {
  calendar_item_id: number;
  alert_offset_secs: number;
  scheduled_at: string;
  stream_id: string;
  playlist_url: string;
  priority: "high" | string;
  expires_at: string;
  published_at: string;
};

export type EdgeAudioReadyMessage = {
  type: "audio_ready";
  payload: { event_id: string; server_time: string } & EdgeAudioReady;
};

export type EdgeAudioFailedMessage = {
  type: "audio_failed";
  payload: { event_id: string; server_time: string } & EdgeAudioFailed;
};

export type EdgeAudioFinalizedMessage = {
  type: "audio_finalized";
  payload: { event_id: string; server_time: string } & EdgeAudioFinalized;
};

export type EdgeCalendarAlertReadyMessage = {
  type: "calendar_alert_ready";
  payload: { event_id: string; server_time: string } & EdgeCalendarAlertReady;
};

export type EdgeAudioEvent =
  | EdgeAudioReadyMessage
  | EdgeAudioFailedMessage
  | EdgeAudioFinalizedMessage
  | EdgeCalendarAlertReadyMessage;

export type EdgeTranslatedNewsItemMessage = {
  type: "translated_news_item";
  payload: {
    event_id: string;
    server_time: string;
    item: TranslatedNewsItem;
  };
};

export type EdgeLiveSnapshot = {
  latest_event_id: string | null;
  now_playing: EdgeNowPlaying | null;
  audio_state: EdgeAudioState | null;
  translated_news_items: EdgeTranslatedNewsItemMessage[];
  audio_events: EdgeAudioEvent[];
  source_statuses?: PublicSourceStatus[];
  calendar_day_summary?: PublicCalendarDaySummaryStatus | null;
};

export type EdgeLiveTicketResponse = {
  ticket: string;
  expires_at: string;
  ttl_secs: number;
};

export type HlsAccessTicketResponse = {
  enabled: boolean;
  ticket: string | null;
  expires_at: string | null;
  ttl_secs: number | null;
};

export type EdgeLiveWsMessage =
  | {
      type: "hello";
      payload: {
        protocol_version: number;
        server_time: string;
        snapshot: EdgeLiveSnapshot;
      };
    }
  | {
      type: "heartbeat";
      payload: {
        server_time: string;
        sources?: PublicSourceStatus[];
        calendar_day_summary?: PublicCalendarDaySummaryStatus | null;
      };
    }
  | {
      type: "news_card";
      payload: { event_id: string; server_time: string; card: EdgeNewsCard };
    }
  | EdgeTranslatedNewsItemMessage
  | {
      type: "now_playing";
      payload: {
        event_id: string;
        server_time: string;
        now_playing: EdgeNowPlaying | null;
      };
    }
  | {
      type: "audio_state";
      payload: { event_id: string; server_time: string; state: EdgeAudioState };
    }
  | EdgeAudioEvent
  | {
      type: "system_notice";
      payload: {
        event_id: string;
        server_time: string;
        severity: EdgeLiveNoticeSeverity;
        message: string;
        code?: string | null;
        detail?: string | null;
      };
    }
  | {
      type: "source_status";
      payload: {
        event_id: string;
        server_time: string;
        sources: PublicSourceStatus[];
        calendar_day_summary?: PublicCalendarDaySummaryStatus | null;
      };
    }
  | { type: "lagged"; payload: { server_time: string; skipped: number } };

export type TestRunAccepted = {
  run_id: string;
  accepted_at: string;
};

export type TestTraceEvent = {
  run_id: string;
  stage: string;
  status: string;
  message: string;
  detail?: string | null;
  timestamp: string;
};

export type TranslationHistoryResponse = {
  items: TranslatedNewsItem[];
  has_more: boolean;
  next_before_id: number | null;
};

export type HlsReplayTodayMetadata = {
  enabled: boolean;
  available: boolean;
  local_day: string;
  playlist_url: string;
  segment_count: number;
  first_sequence: number | null;
  latest_sequence: number | null;
  updated_at: string;
};

export type DailySummaryToday = {
  local_day: string;
  status:
    | "missing"
    | "pending"
    | "summarized"
    | "audio_generating"
    | "audio_ready"
    | "error";
  summary_kind: "ai" | "fallback" | null;
  summary_id: number | null;
  title: string | null;
  daily_context_text: string | null;
  important_news_summary_text: string | null;
  summary_text: string | null;
  narration_text: string | null;
  stream_id: string | null;
  playlist_url: string | null;
  news_count: number;
  important_news_count: number;
  source_urls: string[];
  error_message: string | null;
  updated_at: string | null;
};

export type DailySummaryRun = {
  id: number;
  run_id: string;
  local_day: string;
  summary_id: number | null;
  status: string;
  reason: string;
  forced: boolean;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
};

export type DailySummaryRunStep = {
  id: number;
  run_id: string;
  local_day: string;
  stage: string;
  status: string;
  message: string;
  detail: string | null;
  created_at: string;
};

export type DailySummaryRunArtifact = {
  id: number;
  run_id: string;
  local_day: string;
  kind: string;
  label: string;
  content: string;
  created_at: string;
};

export type DailySummaryAdminNewsItem = {
  newsEventId: number;
  translationId: number | null;
  timestamp: string;
  source: string;
  eventKind: string;
  sourceTitle: string;
  sourceBody: string | null;
  translatedTitle: string | null;
  translatedBody: string | null;
  narrationText: string | null;
  sourceUrl: string | null;
  qualification: {
    category: string;
    importance: string;
    summary: string;
    reasoning: string;
    confidence: number | null;
    isImportant: boolean;
    provider: string | null;
    model: string | null;
    qualifiedAt: string;
  } | null;
};

export type AdminDailySummaryToday = {
  localDay: string;
  currentSummary: DailySummaryToday;
  latestRun: DailySummaryRun | null;
  steps: DailySummaryRunStep[];
  artifacts: DailySummaryRunArtifact[];
  newsUsed: DailySummaryAdminNewsItem[];
  sourceUrls: string[];
};

export type AdminDailySummaryWsMessage =
  | {
      type: "hello";
      payload: {
        protocol_version: number;
        server_time: string;
        user_id: string;
      };
    }
  | { type: "snapshot"; payload: AdminDailySummaryToday }
  | { type: "run_started"; payload: DailySummaryRun }
  | { type: "run_updated"; payload: DailySummaryRun }
  | { type: "step"; payload: DailySummaryRunStep }
  | { type: "artifact"; payload: DailySummaryRunArtifact }
  | { type: "summary_updated"; payload: DailySummaryToday }
  | { type: "heartbeat"; payload: { server_time: string } }
  | { type: "error"; payload: { message: string } };

export type CalendarDayState = {
  localDay: string;
  calendarIngestionStatus: string;
  calendarSummaryStatus: string;
  calendarIngestedAt: string | null;
  calendarContentHash: string | null;
  summaryContentHash: string | null;
  lastRunId: string | null;
  lastError: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CalendarDayRun = {
  id: number;
  runId: string;
  localDay: string;
  workflowKind: string;
  status: string;
  reason: string | null;
  attempt: number;
  errorMessage: string | null;
  n8nWorkflowId: string | null;
  n8nWorkflowName: string | null;
  n8nExecutionId: string | null;
  startedAt: string;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CalendarDayRunStep = {
  id: number;
  runId: string;
  localDay: string;
  stage: string;
  status: string;
  message: string;
  detail: string | null;
  createdAt: string;
};

export type CalendarDayRunArtifact = {
  id: number;
  runId: string;
  localDay: string;
  kind: string;
  label: string;
  contentJson: string;
  createdAt: string;
};

export type CalendarDayItem = {
  id: number;
  localDay: string;
  eventTime: string | null;
  sourceTitle: string;
  translatedTitle: string;
  ttsText: string;
  country: string | null;
  currency: string | null;
  forecastValue: string | null;
  previousValue: string | null;
  actualValue: string | null;
  sourceImportance: string | null;
  aiImportance: string | null;
  finalImportance: string;
  category: string;
  reasoning: string | null;
  confidence: number | null;
  contentHash: string;
  sourcesJson: string;
  createdAt: string;
  updatedAt: string;
};

export type CalendarDayAlert = {
  id: number;
  localDay: string;
  calendarItemId: number;
  alertOffsetSecs: number;
  scheduledAt: string;
  status: string;
  streamId: string | null;
  playlistUrl: string | null;
  publishedAt: string | null;
  expiresAt: string | null;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CalendarDayItemAudio = {
  id: number;
  calendarItemId: number;
  audioKind: string;
  status: string;
  streamId: string | null;
  playlistUrl: string | null;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CalendarDaySummary = {
  id: number;
  localDay: string;
  title: string;
  summaryText: string;
  narrationText: string;
  contentHash: string;
  status: string;
  highEventCount: number;
  provider: string | null;
  model: string | null;
  streamId: string | null;
  playlistUrl: string | null;
  playlistPath: string | null;
  immutablePlaylistUrl: string | null;
  currentPlaylistUrl: string | null;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CalendarDaySummaryToday = {
  localDay: string;
  status:
    | "missing"
    | "pending"
    | "audio_generating"
    | "audio_ready"
    | "error"
    | string;
  summaryId: number | null;
  title: string | null;
  summaryText: string | null;
  narrationText: string | null;
  streamId: string | null;
  playlistUrl: string | null;
  immutablePlaylistUrl: string | null;
  currentPlaylistUrl: string | null;
  highEventCount: number;
  errorMessage: string | null;
  updatedAt: string | null;
};

export type CalendarDayToday = {
  localDay: string;
  state: CalendarDayState;
  itemCount: number;
  highItemCount: number;
  items: CalendarDayItem[];
  summary: CalendarDaySummaryToday | null;
};

export type AdminCalendarDayToday = {
  localDay: string;
  state: CalendarDayState;
  latestRun: CalendarDayRun | null;
  runs: CalendarDayRun[];
  hasActivePendingRun: boolean;
  activePendingRunId: string | null;
  steps: CalendarDayRunStep[];
  artifacts: CalendarDayRunArtifact[];
  items: CalendarDayItem[];
  audios: CalendarDayItemAudio[];
  summary: CalendarDaySummary | null;
  alerts: CalendarDayAlert[];
  automaticWindowStart: string;
  automaticWindowEnd: string;
  automaticRetryBaseSecs: number;
  automaticWindowStatus: string;
};

export type AdminCalendarWeekRun = {
  weekStart: string;
  weekEnd: string;
  days: string[];
  triggeredDays: string[];
  skippedDays: string[];
  mode?: "sequential_memory";
  sequenceId?: string | null;
  activeDay?: string | null;
  activeRunId?: string | null;
  queuedDays?: string[];
};

export type AdminCalendarDayWsMessage =
  | {
      type: "hello";
      payload: {
        protocol_version: number;
        server_time: string;
        user_id: string;
      };
    }
  | { type: "snapshot"; payload: AdminCalendarDayToday }
  | { type: "state_updated"; payload: CalendarDayState }
  | { type: "run_started"; payload: CalendarDayRun }
  | { type: "run_updated"; payload: CalendarDayRun }
  | { type: "step"; payload: CalendarDayRunStep }
  | { type: "artifact"; payload: CalendarDayRunArtifact }
  | { type: "items_updated"; payload: CalendarDayItem[] }
  | { type: "summary_updated"; payload: CalendarDaySummary }
  | { type: "alert_updated"; payload: CalendarDayAlert }
  | { type: "heartbeat"; payload: { server_time: string } }
  | { type: "error"; payload: { message: string } };

export type FinancialJuiceRuntimeStatus = {
  source: string;
  configured_enabled: boolean;
  enabled: boolean;
  disabled_reason: string | null;
};

export type FinancialJuiceBrowserRuntimeStatus = {
  source: string;
  configured_enabled: boolean;
  primary_enabled: boolean;
  fallback_active: boolean;
  ingest_token_configured: boolean;
  status: "ok" | "blocked" | "degraded" | "stopped";
  message: string | null;
  updated_at: string | null;
  effective_enabled: boolean;
  service_accessible: boolean;
  main_feed_available: boolean;
  stale: boolean;
  effective_message: string | null;
};

export type XRuntimeStatus = {
  source: string;
  configured_enabled: boolean;
  enabled: boolean;
  disabled_reason: string | null;
  bearer_token_configured: boolean;
  persistence_available: boolean;
  account_usernames: string[];
  keywords: string[];
  updated_at: string | null;
};

export type TranslationGlossaryEntry = {
  id: number;
  source_term: string;
  translated_term: string;
  spoken_term: string | null;
  source_language: "en";
  target_language: "fr";
  match_mode: "exact" | "word_boundary" | "pattern";
  case_sensitive: boolean;
  priority: number;
  enabled: boolean;
  scope: "full_pipeline" | "spoken_only";
  notes: string | null;
  created_at: string;
  updated_at: string;
};

export type TranslationGlossaryListResponse = {
  items: TranslationGlossaryEntry[];
};

export type TranslationGlossaryReapplyResponse = {
  local_day: string;
  local_days: string[];
  targeted_total: number;
  reprocessed_total: number;
  display_repaired_total: number;
  errors: string[];
};

export type TranslationGlossarySystemRule = {
  id: string;
  family: string;
  label: string;
  input_pattern: string;
  output_pattern: string;
  description: string;
  examples: string[];
};

export type TranslationGlossarySystemRulesResponse = {
  items: TranslationGlossarySystemRule[];
};

export type TranslationGlossaryPreviewRequest = {
  input_text: string;
  input_stage: "source_en" | "display_fr";
  draft_entry?: TranslationGlossaryCreatePayload;
};

export type TranslationGlossaryPreviewResponse = {
  display_text: string;
  spoken_text: string;
  normalized_tts_text: string;
  glossary_hits: number;
  warnings: string[];
};

export type TranslationGlossaryCreatePayload = {
  source_term: string;
  translated_term: string;
  spoken_term: string | null;
  source_language: "en";
  target_language: "fr";
  match_mode: "exact" | "word_boundary" | "pattern";
  case_sensitive: boolean;
  priority: number;
  enabled: boolean;
  scope: "full_pipeline" | "spoken_only";
  notes: string | null;
};

export type TranslationGlossaryUpdatePayload =
  Partial<TranslationGlossaryCreatePayload>;

export type NewsBlacklistMatchMode = "partial" | "exact";
export type NewsBlacklistTargetScope = "title" | "content" | "title_content";
export type NewsBlacklistAction = "reject_news" | "strip_content";

export type NewsBlacklistEntry = {
  id: number;
  term: string;
  match_mode: NewsBlacklistMatchMode;
  target_scope: NewsBlacklistTargetScope;
  enabled: boolean;
  notes: string | null;
  created_at: string;
  updated_at: string;
};

export type NewsBlacklistListResponse = {
  items: NewsBlacklistEntry[];
};

export type NewsBlacklistCreatePayload = {
  term: string;
  match_mode: NewsBlacklistMatchMode;
  target_scope: NewsBlacklistTargetScope;
  enabled: boolean;
  notes: string | null;
};

export type NewsBlacklistUpdatePayload = Partial<NewsBlacklistCreatePayload>;

export type NewsBlacklistHit = {
  id: number;
  blacklist_entry_id: number | null;
  term: string;
  match_mode: NewsBlacklistMatchMode | string;
  target_scope: NewsBlacklistTargetScope | string;
  action: NewsBlacklistAction | string;
  source: string;
  event_kind: string;
  matched_field: "title" | "body" | string;
  matched_text: string;
  source_title: string;
  source_body_excerpt: string | null;
  raw_payload_ref: string | null;
  signature: string;
  created_at: string;
};

export type NewsBlacklistHitsResponse = {
  items: NewsBlacklistHit[];
};

export type NewsBlacklistReapplyResponse = {
  local_day: string;
  local_days: string[];
  targeted_total: number;
  rejected_total: number;
  stripped_total: number;
  reprocessed_total: number;
  errors: string[];
};

export type NewsTruncationAudit = {
  id: number;
  source: string;
  field_name: string;
  marker: string;
  source_title: string;
  field_excerpt: string;
  raw_payload_ref: string | null;
  signature: string;
  created_at: string;
};

export type NewsTruncationAuditsResponse = {
  items: NewsTruncationAudit[];
};

export type AdminLogDay = {
  day: string;
  size_bytes: number;
};

export type AdminLogDaysResponse = {
  days: AdminLogDay[];
};

export type AdminLogLine = {
  cursor: number;
  level: string | null;
  text: string;
};

export type AdminLogPage = {
  day: string;
  items: AdminLogLine[];
  has_more: boolean;
  next_cursor: number | null;
};

export type AdminLogTailSnapshot = {
  day: string;
  items: AdminLogLine[];
  next_cursor: number;
  file_size_bytes: number;
};

export type AdminLogTailAppend = {
  day: string;
  items: AdminLogLine[];
  next_cursor: number;
  file_size_bytes: number;
};

export type IncomingAdminLogWsMessage =
  | {
      type: "hello";
      payload: {
        protocol_version: number;
        server_time: string;
        user_id: string;
      };
    }
  | { type: "snapshot"; payload: AdminLogTailSnapshot }
  | { type: "append"; payload: AdminLogTailAppend }
  | {
      type: "heartbeat";
      payload: { server_time: string; sources?: PublicSourceStatus[] };
    }
  | { type: "error"; payload: { message: string } };

export type UserThemePreference = "light" | "dark";

export type UserPreferences = {
  preferred_tts_voice: string | null;
  theme: UserThemePreference | null;
};

export type CurrentUserProfileResponse = {
  id: string;
  email: string | null;
  role: string;
  preferences: UserPreferences;
  created_at: string;
  updated_at: string;
};

export type IncomingNewsWsMessage =
  | {
      type: "hello";
      payload: {
        protocol_version: number;
        broadcast_buffer?: number;
        server_time?: string;
        sources?: PublicSourceStatus[];
        calendar_day_summary?: PublicCalendarDaySummaryStatus | null;
      };
    }
  | {
      type: "heartbeat";
      payload: {
        server_time: string;
        sources?: PublicSourceStatus[];
        calendar_day_summary?: PublicCalendarDaySummaryStatus | null;
      };
    }
  | { type: "translated_news_item"; payload: TranslatedNewsItem }
  | { type: "lagged"; payload: { skipped: number } };

export type IncomingLiveMetadataMessage =
  | IncomingNewsWsMessage
  | EdgeLiveWsMessage;

export type IncomingTraceWsMessage =
  | { type: "hello"; payload: { protocol_version: number } }
  | { type: "heartbeat"; payload: { server_time: string } }
  | { type: "test_run_accepted"; payload: TestRunAccepted }
  | { type: "test_trace"; payload: TestTraceEvent }
  | { type: "lagged"; payload: { skipped: number } }
  | { type: "error"; payload: { message: string } };

export type AudioTransportMode = "idle" | "hls";

export type HlsPlaybackMode =
  | "idle"
  | "clip"
  | "replay_today"
  | "daily_summary"
  | "calendar_summary";

export type AudioPlaybackEngine =
  | "unknown"
  | "hls_js"
  | "hls_native";
