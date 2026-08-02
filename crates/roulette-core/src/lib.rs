//! Deterministic rules engine for Russian Roulette.
//!
//! This crate owns command validation, state transitions, explicit random
//! streams, domain event generation, and player-view projection. It must not
//! access networks, databases, files, system time, or external language models.

#![forbid(unsafe_code)]
