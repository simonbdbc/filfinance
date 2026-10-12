//! Taches de retention applicative pour garder SQLite borne.

mod schedule;
mod worker;

#[cfg(test)]
pub(in crate::features::news) use schedule::duration_until_next_daily_run;
pub use worker::spawn;
