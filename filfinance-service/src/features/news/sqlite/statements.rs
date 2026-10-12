//! Projections SQL communes du store SQLite news.

mod events;
mod translations;

pub(super) use events::{NEWS_EVENT_QUALIFIED_SELECT, NEWS_EVENT_SELECT};
pub(super) use translations::{NEWS_TRANSLATION_HISTORY_SELECT, NEWS_TRANSLATION_SELECT};
