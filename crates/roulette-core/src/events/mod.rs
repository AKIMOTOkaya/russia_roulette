//! Event system engine for Russian Roulette.
//!
//! Provides deterministic BFS event tree resolution, dynamic dampening,
//! static event registry, and contextual event pools.

pub mod catalog;
pub mod pipeline;
pub mod pools;

pub use catalog::{EventDef, EventRegistry};
pub use pipeline::{EventNode, EventTreePipeline, PipelineOutcome};
pub use pools::{EventPool, PoolEntry, TriggerPoint, resolve_pool};
