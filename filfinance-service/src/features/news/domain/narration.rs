//! Construction et normalisation du texte de narration news.

mod clock;
mod legacy;
mod prefix;

// La facade garde les chemins `news::domain::*` tout en isolant les règles internes.
pub use clock::format_clock_label_for_offset;
pub use legacy::normalize_news_narration_prefix;
pub use prefix::{
    build_news_narration_prefix, build_news_narration_prefix_from_timestamp,
    financial_juice_importance_from_news_level, news_level_contains_active_word,
};
