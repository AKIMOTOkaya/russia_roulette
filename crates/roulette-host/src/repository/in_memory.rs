//! Thread-safe in-memory repository implementation.

#![forbid(unsafe_code)]

use std::sync::RwLock;

use roulette_domain::{RoomId, RoomSummary, TabId};

use crate::error::RepositoryError;
use crate::game::Game;
use crate::repository::GameRepository;

/// In-memory thread-safe storage for games.
#[derive(Debug, Default)]
pub struct InMemoryGameRepository {
    rooms: RwLock<Vec<Game>>,
}

impl InMemoryGameRepository {
    /// Constructs a new empty in-memory repository.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(Vec::new()),
        }
    }
}

impl GameRepository for InMemoryGameRepository {
    fn find_by_id(&self, id: &RoomId) -> Result<Option<Game>, RepositoryError> {
        let guard = self
            .rooms
            .read()
            .map_err(|e| RepositoryError::Storage(format!("lock error: {e}")))?;
        Ok(guard.iter().find(|room| room.id == *id).cloned())
    }

    fn find_by_tab(&self, tab_id: &TabId) -> Result<Option<Game>, RepositoryError> {
        let guard = self
            .rooms
            .read()
            .map_err(|e| RepositoryError::Storage(format!("lock error: {e}")))?;
        Ok(guard.iter().find(|room| room.has_tab(tab_id)).cloned())
    }

    fn save(&self, game: &Game) -> Result<(), RepositoryError> {
        let mut guard = self
            .rooms
            .write()
            .map_err(|e| RepositoryError::Storage(format!("lock error: {e}")))?;
        if let Some(existing) = guard.iter_mut().find(|r| r.id == game.id) {
            *existing = game.clone();
        } else {
            guard.push(game.clone());
        }
        Ok(())
    }

    fn delete(&self, id: &RoomId) -> Result<bool, RepositoryError> {
        let mut guard = self
            .rooms
            .write()
            .map_err(|e| RepositoryError::Storage(format!("lock error: {e}")))?;
        let initial_len = guard.len();
        guard.retain(|r| r.id != *id);
        Ok(guard.len() < initial_len)
    }

    fn list_summaries(&self, for_tab: &TabId) -> Result<Vec<RoomSummary>, RepositoryError> {
        let guard = self
            .rooms
            .read()
            .map_err(|e| RepositoryError::Storage(format!("lock error: {e}")))?;
        Ok(guard.iter().map(|r| r.summary(for_tab)).collect())
    }

    fn all_games(&self) -> Result<Vec<Game>, RepositoryError> {
        let guard = self
            .rooms
            .read()
            .map_err(|e| RepositoryError::Storage(format!("lock error: {e}")))?;
        Ok(guard.clone())
    }
}
