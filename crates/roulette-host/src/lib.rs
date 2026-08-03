//! In-memory local lobby, room ownership, and bot orchestration.
//!
//! The host owns multiple rooms, associates human players with browser-tab
//! identities, validates room permissions, and advances bots through the same
//! deterministic Core command path used by humans. It contains no HTTP, file,
//! database, or process-lifecycle code.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::{Duration, Instant};

use roulette_core::{CoreError, GameEngine};
use roulette_domain::{
    CreateGameConfig, GameState, GameStatus, GameView, LobbyView, PlayerCommand, PlayerId,
    PlayerKind, PlayerSetup, RoomId, RoomMemberId, RoomMemberView, RoomPhase, RoomSummary,
    RoomView, TabId,
};

const MIN_MEMBERS: usize = 3;
const MAX_MEMBERS: usize = 6;
const MAX_AUTOMATED_COMMANDS: usize = 4_096;
const ROOM_CODE_LENGTH: usize = 5;
const ROOM_CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Inputs used to create a room and join its owner tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRoomConfig {
    /// Requesting browser-tab identity.
    pub tab_id: TabId,
    /// Room display name.
    pub room_name: String,
    /// Owner display name.
    pub player_name: String,
    /// Optional case-sensitive join password.
    pub password: Option<String>,
}

/// Inputs used to join an existing waiting room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRoomConfig {
    /// Requesting browser-tab identity.
    pub tab_id: TabId,
    /// Case-insensitive five-character room identifier.
    pub room_id: String,
    /// Human display name.
    pub player_name: String,
    /// Password supplied by the joining human.
    pub password: Option<String>,
}

/// Errors produced by local lobby and match orchestration.
#[derive(Debug)]
pub enum HostError {
    /// A tab, room, member, or display name was malformed.
    InvalidInput(&'static str),
    /// Requested room does not exist.
    RoomNotFound,
    /// Requesting tab is not a member of the room.
    NotRoomMember,
    /// Requesting tab already belongs to another room.
    AlreadyInRoom,
    /// Room password did not match.
    IncorrectRoomPassword,
    /// Room is full.
    RoomFull,
    /// Operation is only valid while waiting for players.
    RoomNotWaiting,
    /// Operation requires the current room owner.
    OwnerRequired,
    /// Requested member does not exist or has the wrong controller kind.
    InvalidMember,
    /// Match cannot start until the room contains three to six members.
    InvalidMemberCount,
    /// Match is not currently running.
    MatchNotRunning,
    /// Browser submitted an obsolete or future revision.
    RevisionConflict {
        /// Revision expected by the host.
        expected: u64,
        /// Revision supplied by the browser.
        actual: u64,
    },
    /// Browser attempted to act for another human or while a bot owned the turn.
    HumanDoesNotOwnTurn,
    /// Automated play exceeded the safety budget.
    AutomationLimitExceeded,
    /// Deterministic Core rejected an operation.
    Core(CoreError),
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "invalid input: {message}"),
            Self::RoomNotFound => formatter.write_str("room not found"),
            Self::NotRoomMember => formatter.write_str("tab is not a room member"),
            Self::AlreadyInRoom => formatter.write_str("tab already belongs to a room"),
            Self::IncorrectRoomPassword => formatter.write_str("incorrect room password"),
            Self::RoomFull => formatter.write_str("room is full"),
            Self::RoomNotWaiting => formatter.write_str("room is not waiting for players"),
            Self::OwnerRequired => formatter.write_str("room owner permission required"),
            Self::InvalidMember => formatter.write_str("invalid room member"),
            Self::InvalidMemberCount => formatter.write_str("a match requires 3 to 6 room members"),
            Self::MatchNotRunning => formatter.write_str("match is not running"),
            Self::RevisionConflict { expected, actual } => write!(
                formatter,
                "revision conflict: expected {expected}, received {actual}"
            ),
            Self::HumanDoesNotOwnTurn => {
                formatter.write_str("this tab does not own the current player turn")
            }
            Self::AutomationLimitExceeded => {
                formatter.write_str("automated bot play exceeded its safety limit")
            }
            Self::Core(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for HostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CoreError> for HostError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

/// Multiple in-memory rooms hosted by one local process.
#[derive(Debug)]
pub struct LocalLobby {
    rooms: Vec<Room>,
    room_code_rng: u64,
}

impl LocalLobby {
    /// Creates an empty lobby with an explicit room-code seed.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self {
            rooms: Vec::new(),
            room_code_rng: seed,
        }
    }

    /// Returns lobby cards customized for one tab.
    #[must_use]
    pub fn view(&self, tab_id: &TabId) -> LobbyView {
        LobbyView {
            current_room_id: self.room_for_tab(tab_id).map(|room| room.id.clone()),
            rooms: self
                .rooms
                .iter()
                .map(|room| RoomSummary {
                    id: room.id.clone(),
                    name: room.name.clone(),
                    phase: room.phase,
                    human_count: room.human_count(),
                    bot_count: room.bot_count(),
                    capacity: MAX_MEMBERS,
                    password_required: room.password.is_some(),
                    is_member: room.has_tab(tab_id),
                })
                .collect(),
        }
    }

    /// Refreshes the activity timestamp for a human tab when it is in a room.
    pub fn heartbeat(&mut self, tab_id: &TabId, now: Instant) {
        if let Some(member) = self
            .rooms
            .iter_mut()
            .flat_map(|room| room.members.iter_mut())
            .find(|member| member.tab_id() == Some(tab_id))
        {
            member.last_seen = Some(now);
        }
    }

    /// Removes inactive human tabs and dissolves rooms with no humans.
    pub fn reap_inactive(&mut self, now: Instant, timeout: Duration) {
        let stale_tabs = self
            .rooms
            .iter()
            .flat_map(|room| {
                room.members.iter().filter_map(|member| {
                    let tab_id = member.tab_id()?;
                    let last_seen = member.last_seen?;
                    now.checked_duration_since(last_seen)
                        .is_some_and(|elapsed| elapsed >= timeout)
                        .then(|| tab_id.clone())
                })
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
    /// Returns an error for malformed input or when the tab already belongs to
    /// a room.
    pub fn create_room(
        &mut self,
        config: &CreateRoomConfig,
        now: Instant,
    ) -> Result<RoomView, HostError> {
        validate_tab_id(&config.tab_id)?;
        validate_name(&config.room_name, 40, "room name")?;
        validate_name(&config.player_name, 24, "player name")?;
        let password = normalize_password(config.password.as_deref())?;
        if self.room_for_tab(&config.tab_id).is_some() {
            return Err(HostError::AlreadyInRoom);
        }
        let room_id = self.generate_room_id()?;
        let owner_member_id = human_member_id(&config.tab_id);
        self.rooms.push(Room {
            id: room_id.clone(),
            name: config.room_name.trim().to_owned(),
            password,
            owner_member_id,
            members: vec![RoomMember::human(
                &config.tab_id,
                config.player_name.trim(),
                now,
            )],
            phase: RoomPhase::Waiting,
            game: None,
            next_bot_number: 1,
        });
        self.room_view(&room_id.0, &config.tab_id)
    }

    /// Joins a waiting room after a case-insensitive room-code lookup.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed input, membership conflicts, an incorrect
    /// password, a full room, or a room that already started.
    pub fn join_room(
        &mut self,
        config: &JoinRoomConfig,
        now: Instant,
    ) -> Result<RoomView, HostError> {
        validate_tab_id(&config.tab_id)?;
        validate_name(&config.player_name, 24, "player name")?;
        if self.room_for_tab(&config.tab_id).is_some() {
            return Err(HostError::AlreadyInRoom);
        }
        let room_id = normalize_room_id(&config.room_id)?;
        let room = self.room_mut(&room_id)?;
        if room.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        if room.members.len() >= MAX_MEMBERS {
            return Err(HostError::RoomFull);
        }
        if room.password.as_deref() != config.password.as_deref() {
            return Err(HostError::IncorrectRoomPassword);
        }
        room.members.push(RoomMember::human(
            &config.tab_id,
            config.player_name.trim(),
            now,
        ));
        room.view(&config.tab_id)
    }

    /// Returns one room for a human member tab.
    ///
    /// # Errors
    ///
    /// Returns an error when the room is missing or the tab is not a member.
    pub fn room_view(&self, room_id: &str, tab_id: &TabId) -> Result<RoomView, HostError> {
        validate_tab_id(tab_id)?;
        self.room(&normalize_room_id(room_id)?)?.view(tab_id)
    }

    /// Removes a human tab. A surviving room transfers ownership; a running
    /// match converts the departed seat to a bot. Rooms with no humans dissolve.
    ///
    /// # Errors
    ///
    /// Returns an error when the tab does not belong to the requested room.
    pub fn leave_room(&mut self, room_id: &str, tab_id: &TabId) -> Result<(), HostError> {
        let normalized = normalize_room_id(room_id)?;
        let room = self.room(&normalized)?;
        if !room.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }
        self.leave_tab_internal(tab_id);
        Ok(())
    }

    /// Adds one bot to a waiting room roster.
    ///
    /// # Errors
    ///
    /// Returns an error when the room is full, already started, or the tab is
    /// not the owner.
    pub fn add_bot(&mut self, room_id: &str, tab_id: &TabId) -> Result<RoomView, HostError> {
        let room = self.owner_room_mut(room_id, tab_id)?;
        if room.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        if room.members.len() >= MAX_MEMBERS {
            return Err(HostError::RoomFull);
        }
        let number = room.next_bot_number;
        room.next_bot_number = room.next_bot_number.saturating_add(1);
        room.members.push(RoomMember::bot(
            RoomMemberId(format!("bot:{number}")),
            format!("Bot {number}"),
        ));
        room.view(tab_id)
    }

    /// Removes one bot from a waiting room roster.
    ///
    /// # Errors
    ///
    /// Returns an error when the room already started, the tab is not owner,
    /// or the target is not a bot.
    pub fn remove_bot(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        let room = self.owner_room_mut(room_id, tab_id)?;
        if room.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        let index = room
            .members
            .iter()
            .position(|member| member.id == *member_id && member.kind == PlayerKind::Bot)
            .ok_or(HostError::InvalidMember)?;
        room.members.remove(index);
        room.view(tab_id)
    }

    /// Transfers room ownership to another human member.
    ///
    /// # Errors
    ///
    /// Returns an error when the actor is not owner or the target is not human.
    pub fn transfer_owner(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        let room = self.owner_room_mut(room_id, tab_id)?;
        let target = room
            .members
            .iter()
            .find(|member| member.id == *member_id && member.kind == PlayerKind::Human)
            .ok_or(HostError::InvalidMember)?;
        room.owner_member_id = target.id.clone();
        room.view(tab_id)
    }

    /// Permanently dissolves a room.
    ///
    /// # Errors
    ///
    /// Returns an error when the requesting tab is not room owner.
    pub fn dissolve_room(&mut self, room_id: &str, tab_id: &TabId) -> Result<(), HostError> {
        let normalized = normalize_room_id(room_id)?;
        let room = self.room(&normalized)?;
        if !room.is_owner(tab_id) {
            return Err(HostError::OwnerRequired);
        }
        self.rooms.retain(|room| room.id != normalized);
        Ok(())
    }

    /// Starts a deterministic match using the current member order.
    ///
    /// # Errors
    ///
    /// Returns an error when the actor is not owner, the room is not waiting,
    /// the member count is invalid, or Core rejects game creation.
    pub fn start_room(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        seed: u64,
    ) -> Result<RoomView, HostError> {
        let room = self.owner_room_mut(room_id, tab_id)?;
        if room.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        if !(MIN_MEMBERS..=MAX_MEMBERS).contains(&room.members.len()) {
            return Err(HostError::InvalidMemberCount);
        }
        let game = HostedMatch::new(seed, &mut room.members)?;
        room.game = Some(game);
        room.phase = RoomPhase::Playing;
        room.view(tab_id)
    }

    /// Applies one human command and automatically advances consecutive bots.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid membership, phase, revision, turn ownership,
    /// automated-play limits, or Core failures.
    pub fn submit_command(
        &mut self,
        room_id: &str,
        tab_id: &TabId,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<RoomView, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let room = self.room_mut(&normalized)?;
        if !room.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }
        if room.phase != RoomPhase::Playing {
            return Err(HostError::MatchNotRunning);
        }
        let game = room.game.as_mut().ok_or(HostError::MatchNotRunning)?;
        game.submit_human_command(tab_id, expected_revision, command)?;
        if matches!(game.state.status, GameStatus::Finished { .. }) {
            room.phase = RoomPhase::Finished;
        }
        room.view(tab_id)
    }

    fn room(&self, room_id: &RoomId) -> Result<&Room, HostError> {
        self.rooms
            .iter()
            .find(|room| room.id == *room_id)
            .ok_or(HostError::RoomNotFound)
    }

    fn room_mut(&mut self, room_id: &RoomId) -> Result<&mut Room, HostError> {
        self.rooms
            .iter_mut()
            .find(|room| room.id == *room_id)
            .ok_or(HostError::RoomNotFound)
    }

    fn room_for_tab(&self, tab_id: &TabId) -> Option<&Room> {
        self.rooms.iter().find(|room| room.has_tab(tab_id))
    }

    fn owner_room_mut(&mut self, room_id: &str, tab_id: &TabId) -> Result<&mut Room, HostError> {
        let normalized = normalize_room_id(room_id)?;
        let room = self.room_mut(&normalized)?;
        if !room.is_owner(tab_id) {
            return Err(HostError::OwnerRequired);
        }
        Ok(room)
    }

    fn generate_room_id(&mut self) -> Result<RoomId, HostError> {
        for _ in 0..1_024 {
            let mut code = String::with_capacity(ROOM_CODE_LENGTH);
            for _ in 0..ROOM_CODE_LENGTH {
                let index = usize::try_from(next_random(&mut self.room_code_rng))
                    .unwrap_or_default()
                    % ROOM_CODE_ALPHABET.len();
                code.push(char::from(ROOM_CODE_ALPHABET[index]));
            }
            let candidate = RoomId(code);
            if self.rooms.iter().all(|room| room.id != candidate) {
                return Ok(candidate);
            }
        }
        Err(HostError::InvalidInput("unable to allocate room id"))
    }

    fn leave_tab_internal(&mut self, tab_id: &TabId) {
        let Some(room_index) = self.rooms.iter().position(|room| room.has_tab(tab_id)) else {
            return;
        };
        let remaining_humans = self.rooms[room_index]
            .members
            .iter()
            .filter(|member| member.kind == PlayerKind::Human && member.tab_id() != Some(tab_id))
            .count();
        if remaining_humans == 0 {
            self.rooms.remove(room_index);
            return;
        }

        let room = &mut self.rooms[room_index];
        let Some(member_index) = room
            .members
            .iter()
            .position(|member| member.tab_id() == Some(tab_id))
        else {
            return;
        };
        let departed_was_owner = room.members[member_index].id == room.owner_member_id;
        if room.phase == RoomPhase::Playing {
            let bot_number = room.next_bot_number;
            room.next_bot_number = room.next_bot_number.saturating_add(1);
            let replacement_id = RoomMemberId(format!("bot:{bot_number}"));
            let replacement_name = format!("{}（托管）", room.members[member_index].name);
            room.members[member_index].id = replacement_id;
            room.members[member_index].name = replacement_name;
            room.members[member_index].kind = PlayerKind::Bot;
            room.members[member_index].tab_id = None;
            room.members[member_index].last_seen = None;
            if let Some(game) = room.game.as_mut() {
                game.convert_human_to_bot(tab_id);
                if matches!(game.state.status, GameStatus::Finished { .. }) {
                    room.phase = RoomPhase::Finished;
                }
            }
        } else {
            room.members.remove(member_index);
        }
        if departed_was_owner
            && let Some(next_owner) = room
                .members
                .iter()
                .find(|member| member.kind == PlayerKind::Human)
        {
            room.owner_member_id = next_owner.id.clone();
        }
    }
}

#[derive(Debug)]
struct Room {
    id: RoomId,
    name: String,
    password: Option<String>,
    owner_member_id: RoomMemberId,
    members: Vec<RoomMember>,
    phase: RoomPhase,
    game: Option<HostedMatch>,
    next_bot_number: u32,
}

impl Room {
    fn has_tab(&self, tab_id: &TabId) -> bool {
        self.members
            .iter()
            .any(|member| member.tab_id() == Some(tab_id))
    }

    fn is_owner(&self, tab_id: &TabId) -> bool {
        self.members
            .iter()
            .any(|member| member.id == self.owner_member_id && member.tab_id() == Some(tab_id))
    }

    fn human_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| member.kind == PlayerKind::Human)
            .count()
    }

    fn bot_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| member.kind == PlayerKind::Bot)
            .count()
    }

    fn view(&self, tab_id: &TabId) -> Result<RoomView, HostError> {
        if !self.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }
        let game = self
            .game
            .as_ref()
            .map(|game| game.view_for(tab_id))
            .transpose()?;
        Ok(RoomView {
            id: self.id.clone(),
            name: self.name.clone(),
            phase: self.phase,
            password: self.password.clone(),
            members: self
                .members
                .iter()
                .map(|member| RoomMemberView {
                    id: member.id.clone(),
                    name: member.name.clone(),
                    kind: member.kind,
                    is_owner: member.id == self.owner_member_id,
                    is_self: member.tab_id() == Some(tab_id),
                    player_id: member.player_id,
                })
                .collect(),
            is_owner: self.is_owner(tab_id),
            game,
        })
    }
}

#[derive(Debug)]
struct RoomMember {
    id: RoomMemberId,
    name: String,
    kind: PlayerKind,
    tab_id: Option<TabId>,
    last_seen: Option<Instant>,
    player_id: Option<PlayerId>,
}

impl RoomMember {
    fn human(tab_id: &TabId, name: &str, now: Instant) -> Self {
        Self {
            id: human_member_id(tab_id),
            name: name.to_owned(),
            kind: PlayerKind::Human,
            tab_id: Some(tab_id.clone()),
            last_seen: Some(now),
            player_id: None,
        }
    }

    fn bot(id: RoomMemberId, name: String) -> Self {
        Self {
            id,
            name,
            kind: PlayerKind::Bot,
            tab_id: None,
            last_seen: None,
            player_id: None,
        }
    }

    const fn tab_id(&self) -> Option<&TabId> {
        self.tab_id.as_ref()
    }
}

#[derive(Debug)]
struct HostedMatch {
    state: GameState,
    human_players: Vec<(TabId, PlayerId)>,
}

impl HostedMatch {
    fn new(seed: u64, members: &mut [RoomMember]) -> Result<Self, HostError> {
        let players = members
            .iter()
            .map(|member| PlayerSetup {
                name: member.name.clone(),
                kind: member.kind,
            })
            .collect();
        let state = GameEngine::create_game(CreateGameConfig { seed, players })?;
        let mut human_players = Vec::new();
        for (index, member) in members.iter_mut().enumerate() {
            let number = u32::try_from(index + 1)
                .map_err(|_| HostError::InvalidInput("player id overflow"))?;
            let player_id = PlayerId(number);
            member.player_id = Some(player_id);
            if let Some(tab_id) = member.tab_id.clone() {
                human_players.push((tab_id, player_id));
            }
        }
        Ok(Self {
            state,
            human_players,
        })
    }

    fn view_for(&self, tab_id: &TabId) -> Result<GameView, HostError> {
        let player_id = self
            .human_players
            .iter()
            .find_map(|(candidate, player_id)| (candidate == tab_id).then_some(*player_id))
            .ok_or(HostError::NotRoomMember)?;
        Ok(GameEngine::project_view(&self.state, player_id)?)
    }

    fn submit_human_command(
        &mut self,
        tab_id: &TabId,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<(), HostError> {
        if expected_revision != self.state.revision {
            return Err(HostError::RevisionConflict {
                expected: self.state.revision,
                actual: expected_revision,
            });
        }
        let player_id = self
            .human_players
            .iter()
            .find_map(|(candidate, player_id)| (candidate == tab_id).then_some(*player_id))
            .ok_or(HostError::NotRoomMember)?;
        if GameEngine::current_player_id(&self.state)? != player_id {
            return Err(HostError::HumanDoesNotOwnTurn);
        }
        GameEngine::apply_command(&mut self.state, player_id, command)?;
        self.advance_bots()
    }

    fn convert_human_to_bot(&mut self, tab_id: &TabId) {
        let Some(index) = self
            .human_players
            .iter()
            .position(|(candidate, _)| candidate == tab_id)
        else {
            return;
        };
        let (_, player_id) = self.human_players.remove(index);
        if let Some(player) = self
            .state
            .players
            .iter_mut()
            .find(|player| player.id == player_id)
        {
            player.kind = PlayerKind::Bot;
        }
        let _ = self.advance_bots();
    }

    fn advance_bots(&mut self) -> Result<(), HostError> {
        for _ in 0..MAX_AUTOMATED_COMMANDS {
            if !matches!(self.state.status, GameStatus::Running) {
                return Ok(());
            }
            let actor_id = GameEngine::current_player_id(&self.state)?;
            let actor = self
                .state
                .players
                .iter()
                .find(|player| player.id == actor_id)
                .ok_or(HostError::Core(CoreError::UnknownPlayer(actor_id)))?;
            if actor.kind == PlayerKind::Human {
                return Ok(());
            }
            let command = GameEngine::choose_random_bot_command(&mut self.state, actor_id)?;
            GameEngine::apply_command(&mut self.state, actor_id, command)?;
        }
        Err(HostError::AutomationLimitExceeded)
    }
}

fn validate_tab_id(tab_id: &TabId) -> Result<(), HostError> {
    let value = tab_id.0.trim();
    if value.is_empty() || value.len() > 128 {
        return Err(HostError::InvalidInput("tab id"));
    }
    Ok(())
}

fn validate_name(value: &str, maximum: usize, label: &'static str) -> Result<(), HostError> {
    let length = value.trim().chars().count();
    if length == 0 || length > maximum {
        return Err(HostError::InvalidInput(label));
    }
    Ok(())
}

fn normalize_password(password: Option<&str>) -> Result<Option<String>, HostError> {
    match password {
        None => Ok(None),
        Some(value) if value.is_empty() || value.chars().count() > 32 => {
            Err(HostError::InvalidInput("room password"))
        }
        Some(value) => Ok(Some(value.to_owned())),
    }
}

fn normalize_room_id(value: &str) -> Result<RoomId, HostError> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.len() != ROOM_CODE_LENGTH
        || !normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return Err(HostError::InvalidInput("room id"));
    }
    Ok(RoomId(normalized))
}

fn human_member_id(tab_id: &TabId) -> RoomMemberId {
    RoomMemberId(format!("human:{}", tab_id.0))
}

fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        lobby
            .transfer_owner(&room.id.0, &owner, &guest_id)
            .expect("transfer");
        assert!(lobby.room_view(&room.id.0, &guest).expect("view").is_owner);
    }

    #[test]
    fn owner_departure_transfers_and_last_human_dissolves_room() {
        let mut lobby = LocalLobby::new(11);
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
        lobby.leave_room(&room.id.0, &owner).expect("owner leave");
        assert!(
            lobby
                .room_view(&room.id.0, &guest)
                .expect("guest view")
                .is_owner
        );
        lobby.leave_room(&room.id.0, &guest).expect("guest leave");
        assert!(lobby.view(&guest).rooms.is_empty());
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
        lobby.add_bot(&room.id.0, &owner).expect("add bot");
        let started = lobby.start_room(&room.id.0, &owner, 99).expect("start");
        let first_revision = started.game.as_ref().expect("game").revision;
        let after_owner = lobby
            .submit_command(&room.id.0, &owner, first_revision, PlayerCommand::Wait)
            .expect("owner command");
        assert_eq!(
            after_owner.game.as_ref().expect("game").current_player_id,
            Some(PlayerId(2))
        );
        let guest_revision = after_owner.game.as_ref().expect("game").revision;
        let after_guest = lobby
            .submit_command(&room.id.0, &guest, guest_revision, PlayerCommand::Wait)
            .expect("guest command");
        assert!(after_guest.game.as_ref().expect("game").revision >= guest_revision + 2);
    }

    #[test]
    fn inactive_tabs_are_reaped_without_dropping_active_owner() {
        let mut lobby = LocalLobby::new(17);
        let owner = tab("owner-tab");
        let started = Instant::now();
        let room = lobby
            .create_room(
                &CreateRoomConfig {
                    tab_id: owner.clone(),
                    room_name: "测试房".to_owned(),
                    player_name: "房主".to_owned(),
                    password: None,
                },
                started,
            )
            .expect("create");
        let guest = tab("guest-tab");
        lobby
            .join_room(
                &JoinRoomConfig {
                    tab_id: guest.clone(),
                    room_id: room.id.0.clone(),
                    player_name: "访客".to_owned(),
                    password: None,
                },
                started,
            )
            .expect("join");
        lobby.heartbeat(&owner, started + Duration::from_secs(10));
        lobby.reap_inactive(started + Duration::from_secs(16), Duration::from_secs(15));
        let view = lobby.room_view(&room.id.0, &owner).expect("room survives");
        assert_eq!(view.members.len(), 1);
        assert!(view.is_owner);
    }
}
