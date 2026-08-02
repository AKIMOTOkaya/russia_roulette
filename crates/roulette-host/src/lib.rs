//! Match hosting and orchestration for Russian Roulette.
//!
//! This crate will own room lifecycle, player slots, per-match command queues,
//! reconnect handling, timeouts, bot scheduling, and persistence ports. Concrete
//! transports and databases remain outside this crate.

#![forbid(unsafe_code)]
