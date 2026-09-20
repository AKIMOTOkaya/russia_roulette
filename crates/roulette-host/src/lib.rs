//! Decoupled domain host and application service architecture for Russian Roulette.
//!
//! # System Layer Hierarchy
//!
//! ```text
//! ┌──────────────────────────────────────┐
//! │                server                │
//! │                                      │
//! │   HTTP        MCP        其他接口层  │
//! │    │           │          │          │
//! │    └───────────┼──────────┘          │
//! │                ▼                     │
//! │           GameService                │
//! │                │                     │
//! │           GameManager                │
//! │                │                     │
//! │             Game                     │
//! │                │                     │
//! │        Rules / State Machine         │
//! │                │                     │
//! │           Repository                 │
//! │                │                     │
//! │        SQLite / PostgreSQL           │
//! │                                      │
//! └──────────────────────────────────────┘
//! ```

#![forbid(unsafe_code)]

pub mod error;
pub mod game;
pub mod manager;
pub mod repository;
pub mod service;
pub mod validation;

use std::sync::Arc;
use std::time::{Duration, Instant};

use roulette_domain::{LobbyView, PlayerCommand, RoomMemberId, RoomView, TabId};

pub use error::{HostError, RepositoryError, ServiceError};
pub use game::{Game, HostedMatch, RoomMember};
pub use manager::GameManager;
pub use repository::{GameRepository, InMemoryGameRepository};
pub use service::{DEFAULT_TAB_TIMEOUT, GameService, ServerSettings, generate_seed};
pub use validation::{
    CreateRoomConfig, JoinRoomConfig, MAX_MEMBERS, MIN_MEMBERS, ROOM_CODE_LENGTH,
};

/// Backward-compatible in-memory lobby facade wrapping `GameManager` and `InMemoryGameRepository`.
#[derive(Debug)]
pub struct LocalLobby {
    manager: GameManager,
}

impl LocalLobby {
    /// Creates an empty lobby backed by an in-memory repository with an explicit room-code seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let repository = Arc::new(InMemoryGameRepository::new());
        Self {
            manager: GameManager::new(repository, seed),
        }
    }

    /// Returns the underlying `GameManager`.
    #[must_use]
    pub const fn manager(&self) -> &GameManager {
        &self.manager
    }

    /// Returns lobby cards customized for one tab.
    #[must_use]
    pub fn view(&self, tab_id: &TabId) -> LobbyView {
        self.manager
            .get_lobby_view(tab_id)
            .unwrap_or_else(|_| LobbyView {
                current_room_id: None,
                rooms: Vec::new(),
            })
    }

    /// Refreshes the activity timestamp for a human tab.
    pub fn heartbeat(&mut self, tab_id: &TabId, now: Instant) {
        self.manager.heartbeat(tab_id, now);
    }

    /// Removes inactive human tabs and dissolves rooms with no humans left.
    pub fn reap_inactive(&mut self, now: Instant, timeout: Duration) {
        self.manager.reap_inactive(now, timeout);
    }

    /// Creates a room and joins the requesting tab as owner.
    ///
    /// # Errors
    ///
    /// Returns an error on validation failure or if tab is already in a room.
    pub fn create_room(
        &mut self,
        config: &CreateRoomConfig,
        now: Instant,
    ) -> Result<RoomView, HostError> {
        self.manager.create_room(config, now)
    }

    /// Joins a waiting room after password check.
    ///
    /// # Errors
    ///
    /// Returns an error on validation, incorrect password, or full room.
    pub fn join_room(
        &mut self,
        config: &JoinRoomConfig,
        now: Instant,
    ) -> Result<RoomView, HostError> {
        self.manager.join_room(config, now)
    }

    /// Returns the room view for a member tab.
    ///
    /// # Errors
    ///
    /// Returns an error if room is not found or tab is not a member.
    pub fn room_view(&self, room_id: &str, tab_id: &TabId) -> Result<RoomView, HostError> {
        self.manager.get_room_view(room_id, tab_id)
    }

    /// Removes a human tab.
    ///
    /// # Errors
    ///
    /// Returns an error if tab does not belong to the room.
    pub fn leave_room(&mut self, room_id: &str, tab_id: &TabId) -> Result<(), HostError> {
        self.manager.leave_room(room_id, tab_id)
    }

    /// Adds one bot to a waiting room roster.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, room is full, or already started.
    pub fn add_bot(&mut self, room_id: &str, tab_id: &TabId) -> Result<RoomView, HostError> {
        self.manager.add_bot(room_id, tab_id)
    }

    /// Removes one bot from a waiting room roster.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner or bot not found.
    pub fn remove_bot(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        self.manager.remove_bot(room_id, tab_id, member_id)
    }

    /// Transfers room ownership to another human member.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner or target is not human.
    pub fn transfer_owner(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        self.manager.transfer_owner(room_id, tab_id, member_id)
    }

    /// Permanently dissolves a room.
    ///
    /// # Errors
    ///
    /// Returns an error if requesting tab is not room owner.
    pub fn dissolve_room(&mut self, room_id: &str, tab_id: &TabId) -> Result<(), HostError> {
        self.manager.dissolve_room(room_id, tab_id)
    }

    /// Starts a deterministic match using the current member order.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, invalid member count, or core creation fails.
    pub fn start_room(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        seed: u64,
    ) -> Result<RoomView, HostError> {
        self.manager.start_room(room_id, tab_id, seed)
    }

    /// Applies one human command.
    ///
    /// # Errors
    ///
    /// Returns an error on revision conflict, turn ownership violation, or core rule failure.
    pub fn submit_command(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<RoomView, HostError> {
        self.manager
            .submit_command(room_id, tab_id, expected_revision, command)
    }

    /// Executes one turn for the current bot (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, match not running, or revision conflict.
    pub fn step_bot(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
    ) -> Result<RoomView, HostError> {
        self.manager.step_bot(room_id, tab_id, expected_revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roulette_domain::{Direction, PlayerId, PlayerKind, RoomPhase};

    fn tab(value: &str) -> TabId {
        TabId(value.to_owned())
    }

    fn create(lobby: &mut LocalLobby, owner: &TabId, password: Option<&str>) -> RoomView {
        lobby
            .create_room(
                &CreateRoomConfig {
                    tab_id: owner.clone(),
                    room_name: "测试房".to_owned(),
                    player_name: "房主".to_owned(),
                    password: password.map(str::to_owned),
                },
                Instant::now(),
            )
            .expect("create room")
    }

    #[test]
    fn room_codes_are_five_characters_and_case_insensitive() {
        let mut lobby = LocalLobby::new(42);
        let owner = tab("owner-tab");
        let room = create(&mut lobby, &owner, None);
        assert_eq!(room.id.0.len(), 5);
        let guest = tab("guest-tab");
        let joined = lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest,
                    room_id: room.id.0.to_ascii_lowercase(),
                    player_name: "访客".to_owned(),
                    password: None,
                },
                Instant::now(),
            )
            .expect("join room");
        assert_eq!(joined.members.len(), 2);
    }

    #[test]
    fn protected_room_requires_password_and_reveals_it_after_join() {
        let mut lobby = LocalLobby::new(7);
        let owner = tab("owner-tab");
        let room = create(&mut lobby, &owner, Some("open-sesame"));
        let guest = tab("guest-tab");
        let wrong = lobby.join_room(
            &JoinRoomConfig {
                tab_id: guest.clone(),
                room_id: room.id.0.clone(),
                player_name: "访客".to_owned(),
                password: Some("wrong".to_owned()),
            },
            Instant::now(),
        );
        assert!(matches!(wrong, Err(HostError::IncorrectRoomPassword)));
        let joined = lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest,
                    room_id: room.id.0,
                    player_name: "访客".to_owned(),
                    password: Some("open-sesame".to_owned()),
                },
                Instant::now(),
            )
            .expect("join protected room");
        assert_eq!(joined.password.as_deref(), Some("open-sesame"));
    }

    #[test]
    fn owner_manages_bots_and_can_transfer_ownership() {
        let mut lobby = LocalLobby::new(9);
        let owner = tab("owner-tab");
        let room = create(&mut lobby, &owner, None);
        let guest = tab("guest-tab");
        let joined = lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest.clone(),
                    room_id: room.id.0.clone(),
                    player_name: "访客".to_owned(),
                    password: None,
                },
                Instant::now(),
            )
            .expect("join");
        let with_bot = lobby.add_bot(&room.id.0, &owner).expect("add bot");
        assert_eq!(with_bot.members.len(), 3);
        let bot_id = with_bot
            .members
            .iter()
            .find(|member| member.kind == PlayerKind::Bot)
            .expect("bot")
            .id
            .clone();
        let without_bot = lobby
            .remove_bot(&room.id.0, &owner, &bot_id)
            .expect("remove bot");
        assert_eq!(without_bot.members.len(), 2);
        let guest_id = joined
            .members
            .iter()
            .find(|member| member.is_self)
            .expect("guest")
            .id
            .clone();
        let transferred = lobby
            .transfer_owner(&room.id.0, &owner, &guest_id)
            .expect("transfer");
        assert!(!transferred.is_owner);
    }

    #[test]
    fn multiple_humans_and_bots_can_start_and_take_turns() {
        let mut lobby = LocalLobby::new(13);
        let owner = tab("owner-tab");
        let room = create(&mut lobby, &owner, None);
        let guest = tab("guest-tab");
        lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest.clone(),
                    room_id: room.id.0.clone(),
                    player_name: "访客".to_owned(),
                    password: None,
                },
                Instant::now(),
            )
            .expect("join");
        lobby.add_bot(&room.id.0, &owner).expect("bot");
        let started = lobby.start_room(&room.id.0, &owner, 77).expect("start");
        assert_eq!(started.phase, RoomPhase::Playing);
        let game = started.game.expect("game");
        let first_actor = game.current_player_id;
        let actor_tab = if started.members[0].player_id == first_actor {
            &owner
        } else if started.members[1].player_id == first_actor {
            &guest
        } else {
            panic!("bot started first in test setup");
        };
        let updated = lobby
            .submit_command(
                &room.id.0,
                actor_tab,
                game.revision,
                PlayerCommand::Move {
                    direction: Direction::Up,
                },
            )
            .expect("command");
        let updated_game = updated.game.expect("updated game");
        assert_eq!(updated_game.revision, game.revision + 1);
    }

    #[test]
    fn solo_mode_host_steps_each_bot_individually() {
        let mut lobby = LocalLobby::new(6);
        let owner = tab("solo-owner");
        let room = create(&mut lobby, &owner, None);
        lobby.add_bot(&room.id.0, &owner).expect("add bot 1");
        lobby.add_bot(&room.id.0, &owner).expect("add bot 2");

        let started = lobby.start_room(&room.id.0, &owner, 6).expect("start");
        let rev0 = started.game.as_ref().expect("game").revision;

        // Human turn (Player 1)
        let after_human = lobby
            .submit_command(&room.id.0, &owner, rev0, PlayerCommand::Wait)
            .expect("human wait");
        let g1 = after_human.game.as_ref().expect("game");
        assert_eq!(g1.current_player_id, Some(PlayerId(2)));
        assert_eq!(g1.revision, rev0 + 1);

        // Step Bot 1 (Player 2)
        let after_bot1 = lobby
            .step_bot(&room.id.0, &owner, g1.revision)
            .expect("step bot 1");
        let g2 = after_bot1.game.as_ref().expect("game");
        assert_eq!(g2.current_player_id, Some(PlayerId(3)));
        assert_eq!(g2.revision, g1.revision + 1);

        // Step Bot 2 (Player 3)
        let after_bot2 = lobby
            .step_bot(&room.id.0, &owner, g2.revision)
            .expect("step bot 2");
        let g3 = after_bot2.game.as_ref().expect("game");
        assert_eq!(g3.current_player_id, Some(PlayerId(1)));
        assert_eq!(g3.revision, g2.revision + 1);
    }

    #[test]
    fn inactive_tabs_are_reaped_without_dropping_active_owner() {
        let mut lobby = LocalLobby::new(21);
        let owner = tab("owner-tab");
        let room = create(&mut lobby, &owner, None);
        let guest = tab("guest-tab");
        let joined = lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest.clone(),
                    room_id: room.id.0.clone(),
                    player_name: "挂机访客".to_owned(),
                    password: None,
                },
                Instant::now()
                    .checked_sub(Duration::from_secs(40))
                    .expect("sub"),
            )
            .expect("join");
        assert_eq!(joined.members.len(), 2);
        lobby.heartbeat(&owner, Instant::now());
        lobby.reap_inactive(Instant::now(), Duration::from_secs(15));
        let view = lobby.room_view(&room.id.0, &owner).expect("owner room");
        assert_eq!(view.members.len(), 1);
    }

    #[test]
    fn owner_departure_transfers_and_last_human_dissolves_room() {
        let mut lobby = LocalLobby::new(31);
        let owner = tab("owner-tab");
        let room = create(&mut lobby, &owner, None);
        let guest = tab("guest-tab");
        lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest.clone(),
                    room_id: room.id.0.clone(),
                    player_name: "新房主".to_owned(),
                    password: None,
                },
                Instant::now(),
            )
            .expect("join");
        lobby.leave_room(&room.id.0, &owner).expect("owner leaves");
        let guest_view = lobby.room_view(&room.id.0, &guest).expect("guest room");
        assert!(guest_view.is_owner);
        lobby.leave_room(&room.id.0, &guest).expect("guest leaves");
        assert!(matches!(
            lobby.room_view(&room.id.0, &guest),
            Err(HostError::RoomNotFound)
        ));
    }

    #[test]
    fn test_game_service_end_to_end_use_cases() {
        let repo = Arc::new(InMemoryGameRepository::new());
        let manager = Arc::new(GameManager::new(repo, 777));
        let service = GameService::new(manager, "secret123".to_string(), false);

        let owner = tab("svc-owner");
        let room = service
            .create_room(&CreateRoomConfig {
                tab_id: owner.clone(),
                room_name: "Service房".to_string(),
                player_name: "ServiceOwner".to_string(),
                password: None,
            })
            .expect("service create room");

        assert_eq!(room.name, "Service房");

        let guest = tab("svc-guest");
        let joined = service
            .join_room(&JoinRoomConfig {
                tab_id: guest.clone(),
                room_id: room.id.0.clone(),
                player_name: "ServiceGuest".to_string(),
                password: None,
            })
            .expect("service join room");

        assert_eq!(joined.members.len(), 2);

        let with_bot = service
            .add_bot(&room.id.0, &owner)
            .expect("service add bot");
        assert_eq!(with_bot.members.len(), 3);

        let started = service
            .start_room(&room.id.0, &owner, Some(101))
            .expect("start");
        assert_eq!(started.phase, RoomPhase::Playing);

        // Test founder auth
        assert!(
            service
                .authenticate_founder(&owner, "secret123")
                .expect("auth")
        );
        let settings = service
            .update_server_settings(&owner, true)
            .expect("update");
        assert!(settings.lan_exposed);
        assert!(settings.founder_authenticated);
    }
}
