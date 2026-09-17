//! Persistence abstraction layer for Game aggregates.
//!
//! Provides the `GameRepository` contract implemented by `InMemoryGameRepository`,
//! and designed for future persistence engines such as `SQLite` or `PostgreSQL`.

#![forbid(unsafe_code)]

pub mod in_memory;

pub use in_memory::InMemoryGameRepository;

use roulette_domain::{RoomId, RoomSummary, TabId};

use crate::error::RepositoryError;
use crate::game::Game;

/// Storage abstraction interface for Game entities.
pub trait GameRepository: std::fmt::Debug + Send + Sync {
    /// Retrieves a game by its room ID.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if storage retrieval fails.
    fn find_by_id(&self, id: &RoomId) -> Result<Option<Game>, RepositoryError>;

    /// Finds a game that currently contains the specified browser tab.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if storage retrieval fails.
    fn find_by_tab(&self, tab_id: &TabId) -> Result<Option<Game>, RepositoryError>;

    /// Inserts or updates a game aggregate in the storage.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if storage write fails.
    fn save(&self, game: &Game) -> Result<(), RepositoryError>;

    /// Removes a game by room ID. Returns true if removed, false if not found.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if storage deletion fails.
    fn delete(&self, id: &RoomId) -> Result<bool, RepositoryError>;

    /// Lists summaries of all rooms tailored for a viewer tab.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if summary listing fails.
    fn list_summaries(&self, for_tab: &TabId) -> Result<Vec<RoomSummary>, RepositoryError>;

    /// Returns snapshots of all games currently in storage (e.g. for batch maintenance/reaping).
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if querying all games fails.
    fn all_games(&self) -> Result<Vec<Game>, RepositoryError>;
}
