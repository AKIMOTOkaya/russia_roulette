//! Application service layer exposing unified game and lobby use cases.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use roulette_domain::{
    LobbyView, PlayerCommand, RoomMemberId, RoomView, TabId, Terrain, TerrainProperties,
};
use serde::{Deserialize, Serialize};

use crate::error::ServiceError;
use crate::manager::GameManager;
use crate::validation::{CreateRoomConfig, JoinRoomConfig};

/// Default inactivity timeout before human tabs are reaped.
pub const DEFAULT_TAB_TIMEOUT: Duration = Duration::from_secs(15);

/// Server runtime settings snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Whether the requesting tab has authenticated as founder.
    pub founder_authenticated: bool,
    /// Whether server is exposed to local area network.
    pub lan_exposed: bool,
}

/// Unified application service facade for HTTP, MCP, and other interfaces.
#[derive(Debug)]
pub struct GameService {
    manager: Arc<GameManager>,
    tab_timeout: Duration,
    founder_password: String,
    founder_tabs: Mutex<HashSet<TabId>>,
    lan_exposed: AtomicBool,
}

impl GameService {
    /// Creates a new `GameService` wrapping the game manager.
    #[must_use]
    pub fn new(manager: Arc<GameManager>, founder_password: String, lan_exposed: bool) -> Self {
        Self {
            manager,
            tab_timeout: DEFAULT_TAB_TIMEOUT,
            founder_password,
            founder_tabs: Mutex::new(HashSet::new()),
            lan_exposed: AtomicBool::new(lan_exposed),
        }
    }

    /// Sets a custom tab timeout duration.
    #[must_use]
    pub fn with_tab_timeout(mut self, timeout: Duration) -> Self {
        self.tab_timeout = timeout;
        self
    }

    /// Returns the underlying `GameManager` reference.
    #[must_use]
    pub fn manager(&self) -> &Arc<GameManager> {
        &self.manager
    }

    /// Renders current lobby view for a tab, refreshing activity.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if query fails.
    pub fn get_lobby_view(&self, tab_id: &TabId) -> Result<LobbyView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self.manager.get_lobby_view(tab_id)?)
    }

    /// Heartbeat signal from a client tab to maintain active session.
    pub fn heartbeat(&self, tab_id: &TabId) {
        self.refresh_tab(tab_id);
    }

    /// Creates a new room with the requesting tab as owner.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if room creation or validation fails.
    pub fn create_room(&self, config: &CreateRoomConfig) -> Result<RoomView, ServiceError> {
        self.refresh_tab(&config.tab_id);
        let now = Instant::now();
        Ok(self.manager.create_room(config, now)?)
    }

    /// Joins an existing waiting room.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if joining fails.
    pub fn join_room(&self, config: &JoinRoomConfig) -> Result<RoomView, ServiceError> {
        let now = Instant::now();
        self.manager.reap_inactive(now, self.tab_timeout);
        Ok(self.manager.join_room(config, now)?)
    }

    /// Retrieves detailed room view for a member tab.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if room does not exist or tab is not a member.
    pub fn get_room_view(&self, room_id: &str, tab_id: &TabId) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self.manager.get_room_view(room_id, tab_id)?)
    }

    /// Leaves a room, returning the updated lobby view for the tab.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if leaving fails.
    pub fn leave_room(&self, room_id: &str, tab_id: &TabId) -> Result<LobbyView, ServiceError> {
        self.manager.leave_room(room_id, tab_id)?;
        self.get_lobby_view(tab_id)
    }

    /// Adds a bot to the room (owner only).
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if adding bot fails.
    pub fn add_bot(&self, room_id: &str, tab_id: &TabId) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self.manager.add_bot(room_id, tab_id)?)
    }

    /// Removes a bot from the room (owner only).
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if removing bot fails.
    pub fn remove_bot(
        &self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self.manager.remove_bot(room_id, tab_id, member_id)?)
    }

    /// Transfers room ownership to another human member.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if transfer fails.
    pub fn transfer_owner(
        &self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self.manager.transfer_owner(room_id, tab_id, member_id)?)
    }

    /// Starts match in a room (owner only).
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if start fails.
    pub fn start_room(
        &self,
        room_id: &str,
        tab_id: &TabId,
        seed: Option<u64>,
    ) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        let actual_seed = seed.unwrap_or_else(generate_seed);
        Ok(self.manager.start_room(room_id, tab_id, actual_seed)?)
    }

    /// Permanently dissolves a room (owner only).
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if dissolve fails.
    pub fn dissolve_room(&self, room_id: &str, tab_id: &TabId) -> Result<LobbyView, ServiceError> {
        self.manager.dissolve_room(room_id, tab_id)?;
        self.get_lobby_view(tab_id)
    }

    /// Applies a human player's action command in a running match.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if command fails or revision conflicts.
    pub fn submit_command(
        &self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self
            .manager
            .submit_command(room_id, tab_id, expected_revision, command)?)
    }

    /// Advances the current bot player turn by one step (owner only).
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if stepping fails or revision conflicts.
    pub fn step_bot(
        &self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
    ) -> Result<RoomView, ServiceError> {
        self.refresh_tab(tab_id);
        Ok(self.manager.step_bot(room_id, tab_id, expected_revision)?)
    }

    /// Authenticates a tab as founder using the temporary founder password.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError::Forbidden` if password does not match.
    pub fn authenticate_founder(
        &self,
        tab_id: &TabId,
        password: &str,
    ) -> Result<bool, ServiceError> {
        if password != self.founder_password {
            return Err(ServiceError::Forbidden("创始人临时密码错误".to_string()));
        }
        let mut guard = self
            .founder_tabs
            .lock()
            .map_err(|_| ServiceError::Internal("创始人会话锁损坏".to_string()))?;
        guard.insert(tab_id.clone());
        Ok(true)
    }

    /// Returns the current server settings from the perspective of a tab.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError` if checking founder status fails.
    pub fn get_server_settings(&self, tab_id: &TabId) -> Result<ServerSettings, ServiceError> {
        Ok(ServerSettings {
            founder_authenticated: self.is_founder(tab_id)?,
            lan_exposed: self.lan_exposed.load(Ordering::Relaxed),
        })
    }

    /// Updates whether LAN exposure is permitted (requires founder authentication).
    ///
    /// # Errors
    ///
    /// Returns `ServiceError::Forbidden` if tab is not an authenticated founder.
    pub fn update_server_settings(
        &self,
        tab_id: &TabId,
        expose_lan: bool,
    ) -> Result<ServerSettings, ServiceError> {
        if !self.is_founder(tab_id)? {
            return Err(ServiceError::Forbidden(
                "请先验证创始人临时密码".to_string(),
            ));
        }
        self.lan_exposed.store(expose_lan, Ordering::Relaxed);
        self.get_server_settings(tab_id)
    }

    /// Checks whether LAN access is currently enabled.
    #[must_use]
    pub fn is_lan_exposed(&self) -> bool {
        self.lan_exposed.load(Ordering::Relaxed)
    }

    /// Checks if a tab has founder status.
    ///
    /// # Errors
    ///
    /// Returns `ServiceError::Internal` if the founder lock is poisoned.
    pub fn is_founder(&self, tab_id: &TabId) -> Result<bool, ServiceError> {
        let guard = self
            .founder_tabs
            .lock()
            .map_err(|_| ServiceError::Internal("创始人会话锁损坏".to_string()))?;
        Ok(guard.contains(tab_id))
    }

    /// Returns static canonical terrain rules and property definitions.
    #[must_use]
    pub fn get_terrain_rules(&self) -> Vec<TerrainProperties> {
        Terrain::all_properties()
    }

    fn refresh_tab(&self, tab_id: &TabId) {
        let now = Instant::now();
        self.manager.reap_inactive(now, self.tab_timeout);
        self.manager.heartbeat(tab_id, now);
    }
}

/// Generates a non-zero pseudo-random seed from current system timestamp.
#[must_use]
pub fn generate_seed() -> u64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_secs() ^ u64::from(duration.subsec_nanos())
}
