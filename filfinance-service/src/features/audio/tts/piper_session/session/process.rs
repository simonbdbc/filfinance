//! Facade du process helper Piper et du protocole JSONL bas niveau.

// `command` echange le JSONL, `start` demarre le child, `shutdown` ferme la session.
mod command;
mod shutdown;
mod start;
