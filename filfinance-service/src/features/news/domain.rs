//! Modeles metier communs au pipeline, a la persistence et aux sorties.

mod daily_summary;
mod event;
mod list;
mod narration;
mod signature;
mod source_status;
mod test_refs;

// Les reexports conservent le chemin historique crate::features::news::domain::* pendant le split.
pub use daily_summary::{
    DailyNewsQualificationDraft, DailyNewsQualificationUpsert, DailySummaryAdminBroadcastEvent,
    DailySummaryDraft, DailySummaryRunDraft, PersistedDailyNewsQualification,
    PersistedDailySummary, PersistedDailySummaryRun, PersistedDailySummaryRunArtifact,
    PersistedDailySummaryRunStep,
};
#[allow(unused_imports)]
pub use event::{EventKind, MacroMetadata, NewsEvent, PersistedNewsEvent};
#[allow(unused_imports)]
pub use list::{NewsListParams, NewsListResponse};
#[allow(unused_imports)]
pub use narration::{
    build_news_narration_prefix, build_news_narration_prefix_from_timestamp,
    financial_juice_importance_from_news_level, format_clock_label_for_offset,
    news_level_contains_active_word, normalize_news_narration_prefix,
};
#[allow(unused_imports)]
pub use signature::signature_for;
#[allow(unused_imports)]
pub(in crate::features::news) use signature::{
    financial_juice_title_key, financial_juice_titles_are_near_duplicates,
    is_financial_juice_family,
};
#[allow(unused_imports)]
pub use source_status::{PublicSourceStatus, SourceStatus};
#[allow(unused_imports)]
pub use test_refs::{TEST_RUN_REF_PREFIX, extract_test_run_id};
