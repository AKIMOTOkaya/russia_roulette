//! Domain management layer coordinating game lifecycle, room short codes, and persistence.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use roulette_domain::{LobbyView, PlayerCommand, RoomId, RoomMemberId, RoomView, TabId};

use crate::error::HostError;
use crate::game::Game;
use crate::repository::GameRepository;
use crate::validation::{
    CreateRoomConfig, JoinRoomConfig, ROOM_CODE_ALPHABET, ROOM_CODE_LENGTH, next_random,
    normalize_password, normalize_room_id, validate_name, validate_tab_id,
};

/// Coordinates multi-room lifecycle and persists games via `GameRepository`.
#[derive(Debug)]
pub struct GameManager {
    repository: Arc<dyn GameRepository>,
    room_code_rng: Mutex<u64>,
}

impl GameManager {
    /// Creates a new game manager backed by the given repository and RNG seed.
    #[must_use]
    pub fn new(repository: Arc<dyn GameRepository>, seed: u64) -> Self {
        Self {
            repository,
            room_code_rng: Mutex::new(seed),
        }
    }

    /// Returns the underlying repository reference.
    #[must_use]
    pub fn repository(&self) -> &Arc<dyn GameRepository> {
        &self.repository
    }

    /// Renders lobby cards customized for one tab.
    ///
    /// # Errors
    ///
    /// Returns an error if repository query fails.
    pub fn get_lobby_view(&self, tab_id: &TabId) -> Result<LobbyView, HostError> {
        let current_room_id = self.repository.find_by_tab(tab_id)?.map(|game| game.id);
        let rooms = self.repository.list_summaries(tab_id)?;
        Ok(LobbyView {
            current_room_id,
            rooms,
        })
    }

    /// Refreshes the activity timestamp for a human tab.
    pub fn heartbeat(&self, tab_id: &TabId, now: Instant) {
        if let Ok(Some(mut game)) = self.repository.find_by_tab(tab_id) {
            game.heartbeat(tab_id, now);
            let _ = self.repository.save(&game);
        }
    }

    /// Removes inactive human tabs and dissolves rooms with no humans left.
    pub fn reap_inactive(&self, now: Instant, timeout: Duration) {
        let Ok(games) = self.repository.all_games() else {
            return;
        };
        let stale_tabs = games
            .iter()
            .flat_map(|game| {
                game.members
                    .iter()
                    .filter(|m| m.is_stale(now, timeout))
                    .filter_map(|m| m.tab_id().cloned())
            })
            .collect::<Vec<_>>();

        for tab_id in stale_tabs {
            self.leave_tab_internal(&tab_id);
        }
    }

    /// Creates a room and joins the requesting tab as owner.
    ///
    /// # Errors
    ///
    /// Returns an error on validation failure or if tab is already in a room.
    pub fn create_room(
        &self,
        config: &CreateRoomConfig,
        now: Instant,
    ) -> Result<RoomView, HostError> {
        validate_tab_id(&config.tab_id)?;
        validate_name(&config.room_name, 40, "room name")?;
        validate_name(&config.player_name, 24, "player name")?;
        let password = normalize_password(config.password.as_deref())?;

        if self.repository.find_by_tab(&config.tab_id)?.is_some() {
            return Err(HostError::AlreadyInRoom);
        }

        let room_id = self.generate_room_id()?;
        let game = Game::new(
            room_id,
            config.room_name.trim().to_owned(),
            password,
            &config.tab_id,
            config.player_name.trim(),
            now,
        );

        self.repository.save(&game)?;
        game.view(&config.tab_id)
    }

    /// Joins an existing waiting room after password check.
    ///
    /// # Errors
    ///
    /// Returns an error on validation, incorrect password, or full room.
    pub fn join_room(&self, config: &JoinRoomConfig, now: Instant) -> Result<RoomView, HostError> {
        validate_tab_id(&config.tab_id)?;
        validate_name(&config.player_name, 24, "player name")?;

        if self.repository.find_by_tab(&config.tab_id)?.is_some() {
            return Err(HostError::AlreadyInRoom);
        }

        let room_id = normalize_room_id(&config.room_id)?;
        let mut game = self
            .repository
            .find_by_id(&room_id)?
            .ok_or(HostError::RoomNotFound)?;

        game.add_human(
            &config.tab_id,
            config.player_name.trim(),
            config.password.as_deref(),
            now,
        )?;

        self.repository.save(&game)?;
        game.view(&config.tab_id)
    }

    /// Returns the room view for a human member tab.
    ///
    /// # Errors
    ///
    /// Returns an error if room is not found or tab is not a member.
    pub fn get_room_view(&self, room_id: &str, tab_id: &TabId) -> Result<RoomView, HostError> {
        validate_tab_id(tab_id)?;
        let normalized = normalize_room_id(room_id)?;
        let game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;
        game.view(tab_id)
    }

    /// Removes a human tab from a room.
    ///
    /// # Errors
    ///
    /// Returns an error if tab does not belong to the room.
    pub fn leave_room(&self, room_id: &str, tab_id: &TabId) -> Result<(), HostError> {
        let normalized = normalize_room_id(room_id)?;
        let game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        if !game.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }

        self.leave_tab_internal(tab_id);
        Ok(())
    }

    /// Adds one bot to a waiting room (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, room full, or already started.
    pub fn add_bot(&self, room_id: &str, tab_id: &TabId) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let mut game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        let view = game.add_bot(tab_id)?;
        self.repository.save(&game)?;
        Ok(view)
    }

    /// Removes one bot from a waiting room (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner or bot not found.
    pub fn remove_bot(
        &self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let mut game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        let view = game.remove_bot(tab_id, member_id)?;
        self.repository.save(&game)?;
        Ok(view)
    }

    /// Transfers room ownership to another human member.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner or target is not human.
    pub fn transfer_owner(
        &self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let mut game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        let view = game.transfer_owner(tab_id, member_id)?;
        self.repository.save(&game)?;
        Ok(view)
    }

    /// Starts a deterministic match in the room (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, invalid member count, or core creation fails.
    pub fn start_room(
        &self,
        room_id: &str,
        tab_id: &TabId,
        seed: u64,
    ) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let mut game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        let view = game.start_match(tab_id, seed)?;
        self.repository.save(&game)?;
        Ok(view)
    }

    /// Permanently dissolves a room (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if tab is not room owner.
    pub fn dissolve_room(&self, room_id: &str, tab_id: &TabId) -> Result<(), HostError> {
        let normalized = normalize_room_id(room_id)?;
        let game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        if !game.is_owner(tab_id) {
            return Err(HostError::OwnerRequired);
        }

        self.repository.delete(&normalized)?;
        Ok(())
    }

    /// Applies one human player command.
    ///
    /// # Errors
    ///
    /// Returns an error on revision conflict, turn ownership violation, or core error.
    pub fn submit_command(
        &self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let mut game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        let view = game.submit_command(tab_id, expected_revision, command)?;
        self.repository.save(&game)?;
        Ok(view)
    }

    /// Steps the active bot player (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error on revision conflict, match not running, or if human owns turn.
    pub fn step_bot(
        &self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
    ) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let mut game = self
            .repository
            .find_by_id(&normalized)?
            .ok_or(HostError::RoomNotFound)?;

        let view = game.step_bot(tab_id, expected_revision)?;
        self.repository.save(&game)?;
        Ok(view)
    }

    fn generate_room_id(&self) -> Result<RoomId, HostError> {
        let mut rng_guard = self
            .room_code_rng
            .lock()
            .map_err(|_| HostError::InvalidInput("rng lock error"))?;

        for _ in 0..1_024 {
            let mut code = String::with_capacity(ROOM_CODE_LENGTH);
            for _ in 0..ROOM_CODE_LENGTH {
                let index = usize::try_from(next_random(&mut rng_guard)).unwrap_or_default()
                    % ROOM_CODE_ALPHABET.len();
                code.push(char::from(ROOM_CODE_ALPHABET[index]));
            }
            let candidate = RoomId(code);
            if self.repository.find_by_id(&candidate)?.is_none() {
                return Ok(candidate);
            }
        }
        Err(HostError::InvalidInput("unable to allocate room id"))
    }

    fn leave_tab_internal(&self, tab_id: &TabId) {
        let Ok(Some(mut game)) = self.repository.find_by_tab(tab_id) else {
            return;
        };

        let has_remaining = game.remove_tab(tab_id);
        if has_remaining {
            let _ = self.repository.save(&game);
        } else {
            let _ = self.repository.delete(&game.id);
        }
    }
}
