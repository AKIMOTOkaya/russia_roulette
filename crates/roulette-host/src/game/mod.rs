//! Game aggregate entity, encapsulating room state, participants, and deterministic match progress.

#![forbid(unsafe_code)]

pub mod member;

use std::time::Instant;

use roulette_core::{CoreError, GameEngine};
use roulette_domain::{
    CreateGameConfig, GameState, GameStatus, GameView, PlayerCommand, PlayerId, PlayerKind,
    PlayerSetup, RoomId, RoomMemberId, RoomMemberView, RoomPhase, RoomSummary, RoomView, TabId,
};

use crate::error::HostError;
use crate::validation::{MAX_MEMBERS, MIN_MEMBERS, human_member_id};

pub use member::RoomMember;

/// A hosted game aggregate entity representing a room and its associated match session.
#[derive(Debug, Clone)]
pub struct Game {
    /// Case-insensitive 5-character unique room identifier.
    pub id: RoomId,
    /// Room display name.
    pub name: String,
    /// Optional password required to join.
    pub password: Option<String>,
    /// Member ID of current room owner.
    pub owner_member_id: RoomMemberId,
    /// Ordered participant roster (humans and bots).
    pub members: Vec<RoomMember>,
    /// Lifecycle phase of the room.
    pub phase: RoomPhase,
    /// Hosted deterministic match instance if currently playing or finished.
    pub match_session: Option<HostedMatch>,
    /// Monotonic counter for assigning bot names/IDs.
    pub next_bot_number: u32,
}

impl Game {
    /// Creates a new game aggregate with the owner seated as first member.
    #[must_use]
    pub fn new(
        id: RoomId,
        name: String,
        password: Option<String>,
        owner_tab: &TabId,
        owner_name: &str,
        now: Instant,
    ) -> Self {
        let owner_member_id = human_member_id(owner_tab);
        let owner_member = RoomMember::human(owner_tab, owner_name, now);
        Self {
            id,
            name,
            password,
            owner_member_id,
            members: vec![owner_member],
            phase: RoomPhase::Waiting,
            match_session: None,
            next_bot_number: 1,
        }
    }

    /// Checks whether a tab belongs to this game's human roster.
    #[must_use]
    pub fn has_tab(&self, tab_id: &TabId) -> bool {
        self.members
            .iter()
            .any(|member| member.tab_id() == Some(tab_id))
    }

    /// Checks whether a tab is the current room owner.
    #[must_use]
    pub fn is_owner(&self, tab_id: &TabId) -> bool {
        self.members
            .iter()
            .any(|member| member.id == self.owner_member_id && member.tab_id() == Some(tab_id))
    }

    /// Counts human members currently in the room.
    #[must_use]
    pub fn human_count(&self) -> usize {
        self.members
            .iter()
            .filter(|member| member.is_human())
            .count()
    }

    /// Counts bot members currently in the room.
    #[must_use]
    pub fn bot_count(&self) -> usize {
        self.members.iter().filter(|member| member.is_bot()).count()
    }

    /// Generates a public room summary tailored for a viewer tab.
    #[must_use]
    pub fn summary(&self, for_tab: &TabId) -> RoomSummary {
        RoomSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            phase: self.phase,
            human_count: self.human_count(),
            bot_count: self.bot_count(),
            capacity: MAX_MEMBERS,
            password_required: self.password.is_some(),
            is_member: self.has_tab(for_tab),
        }
    }

    /// Renders the complete room view for a member tab.
    ///
    /// # Errors
    ///
    /// Returns `HostError::NotRoomMember` if tab is not in the room.
    pub fn view(&self, tab_id: &TabId) -> Result<RoomView, HostError> {
        if !self.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }
        let game = self
            .match_session
            .as_ref()
            .map(|m| m.view_for(tab_id))
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

    /// Updates activity heartbeat for a human tab.
    pub fn heartbeat(&mut self, tab_id: &TabId, now: Instant) {
        if let Some(member) = self.members.iter_mut().find(|m| m.tab_id() == Some(tab_id)) {
            member.touch(now);
        }
    }

    /// Adds a human member to a waiting room after password and capacity validation.
    ///
    /// # Errors
    ///
    /// Returns an error if room is not waiting, is full, or password fails.
    pub fn add_human(
        &mut self,
        tab_id: &TabId,
        player_name: &str,
        password: Option<&str>,
        now: Instant,
    ) -> Result<(), HostError> {
        if self.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        if self.members.len() >= MAX_MEMBERS {
            return Err(HostError::RoomFull);
        }
        if self.password.as_deref() != password {
            return Err(HostError::IncorrectRoomPassword);
        }
        self.members
            .push(RoomMember::human(tab_id, player_name, now));
        Ok(())
    }

    /// Adds a bot to a waiting room (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, not waiting, or room is full.
    pub fn add_bot(&mut self, tab_id: &TabId) -> Result<RoomView, HostError> {
        self.ensure_owner(tab_id)?;
        if self.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        if self.members.len() >= MAX_MEMBERS {
            return Err(HostError::RoomFull);
        }
        let number = self.next_bot_number;
        self.next_bot_number = self.next_bot_number.saturating_add(1);
        self.members.push(RoomMember::bot(
            RoomMemberId(format!("bot:{number}")),
            format!("Bot {number}"),
        ));
        self.view(tab_id)
    }

    /// Removes a bot from a waiting room (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, not waiting, or bot not found.
    pub fn remove_bot(
        &mut self,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        self.ensure_owner(tab_id)?;
        if self.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        let index = self
            .members
            .iter()
            .position(|m| m.id == *member_id && m.is_bot())
            .ok_or(HostError::InvalidMember)?;
        self.members.remove(index);
        self.view(tab_id)
    }

    /// Transfers room ownership to another human member.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner or target is not human.
    pub fn transfer_owner(
        &mut self,
        tab_id: &TabId,
        member_id: &RoomMemberId,
    ) -> Result<RoomView, HostError> {
        self.ensure_owner(tab_id)?;
        let target = self
            .members
            .iter()
            .find(|m| m.id == *member_id && m.is_human())
            .ok_or(HostError::InvalidMember)?;
        self.owner_member_id = target.id.clone();
        self.view(tab_id)
    }

    /// Starts a deterministic match for this room using current member configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, not waiting, invalid member count, or core error.
    pub fn start_match(&mut self, tab_id: &TabId, seed: u64) -> Result<RoomView, HostError> {
        self.ensure_owner(tab_id)?;
        if self.phase != RoomPhase::Waiting {
            return Err(HostError::RoomNotWaiting);
        }
        if !(MIN_MEMBERS..=MAX_MEMBERS).contains(&self.members.len()) {
            return Err(HostError::InvalidMemberCount);
        }
        let match_session = HostedMatch::new(seed, &mut self.members)?;
        self.match_session = Some(match_session);
        self.phase = RoomPhase::Playing;
        self.view(tab_id)
    }

    /// Submits a human player command.
    ///
    /// # Errors
    ///
    /// Returns an error if match not running, revision conflict, or not your turn.
    pub fn submit_command(
        &mut self,
        tab_id: &TabId,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<RoomView, HostError> {
        if !self.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }
        if self.phase != RoomPhase::Playing {
            return Err(HostError::MatchNotRunning);
        }
        let match_session = self
            .match_session
            .as_mut()
            .ok_or(HostError::MatchNotRunning)?;
        match_session.submit_human_command(tab_id, expected_revision, command)?;
        if matches!(match_session.state.status, GameStatus::Finished { .. }) {
            self.phase = RoomPhase::Finished;
        }
        self.view(tab_id)
    }

    /// Executes one single step for the current active bot (owner only).
    ///
    /// # Errors
    ///
    /// Returns an error if not owner, match not running, revision conflict, or a human owns turn.
    pub fn step_bot(
        &mut self,
        tab_id: &TabId,
        expected_revision: u64,
    ) -> Result<RoomView, HostError> {
        if !self.has_tab(tab_id) {
            return Err(HostError::NotRoomMember);
        }
        self.ensure_owner(tab_id)?;
        if self.phase != RoomPhase::Playing {
            return Err(HostError::MatchNotRunning);
        }
        let match_session = self
            .match_session
            .as_mut()
            .ok_or(HostError::MatchNotRunning)?;
        match_session.step_bot(expected_revision)?;
        if matches!(match_session.state.status, GameStatus::Finished { .. }) {
            self.phase = RoomPhase::Finished;
        }
        self.view(tab_id)
    }

    /// Removes a human tab. If playing, converts seat to bot takeover; otherwise removes.
    /// Returns whether any human members remain in the room.
    pub fn remove_tab(&mut self, tab_id: &TabId) -> bool {
        let remaining_humans = self
            .members
            .iter()
            .filter(|m| m.is_human() && m.tab_id() != Some(tab_id))
            .count();
        if remaining_humans == 0 {
            return false;
        }

        let Some(member_index) = self.members.iter().position(|m| m.tab_id() == Some(tab_id))
        else {
            return true;
        };

        let departed_was_owner = self.members[member_index].id == self.owner_member_id;
        if self.phase == RoomPhase::Playing {
            let bot_number = self.next_bot_number;
            self.next_bot_number = self.next_bot_number.saturating_add(1);
            let replacement_id = RoomMemberId(format!("bot:{bot_number}"));
            let replacement_name = format!("{}（托管）", self.members[member_index].name);
            self.members[member_index].id = replacement_id;
            self.members[member_index].name = replacement_name;
            self.members[member_index].kind = PlayerKind::Bot;
            self.members[member_index].tab_id = None;
            self.members[member_index].last_seen = None;
            if let Some(match_session) = self.match_session.as_mut() {
                match_session.convert_human_to_bot(tab_id);
                if matches!(match_session.state.status, GameStatus::Finished { .. }) {
                    self.phase = RoomPhase::Finished;
                }
            }
        } else {
            self.members.remove(member_index);
        }

        if departed_was_owner && let Some(next_owner) = self.members.iter().find(|m| m.is_human()) {
            self.owner_member_id = next_owner.id.clone();
        }

        true
    }

    fn ensure_owner(&self, tab_id: &TabId) -> Result<(), HostError> {
        if !self.is_owner(tab_id) {
            return Err(HostError::OwnerRequired);
        }
        Ok(())
    }
}

/// Orchestrates deterministic `GameEngine` execution, tracking tab-to-player mappings.
#[derive(Debug, Clone)]
pub struct HostedMatch {
    /// Underlying deterministic state machine.
    pub state: GameState,
    /// Mapping of active human browser tabs to Core `PlayerId` identifiers.
    pub human_players: Vec<(TabId, PlayerId)>,
}

impl HostedMatch {
    /// Initializes deterministic match state and assigns player IDs to members.
    ///
    /// # Errors
    ///
    /// Returns an error if core creation fails or player IDs overflow.
    pub fn new(seed: u64, members: &mut [RoomMember]) -> Result<Self, HostError> {
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

    /// Projects game view from the perspective of a member tab.
    ///
    /// # Errors
    ///
    /// Returns `HostError::NotRoomMember` if tab is not associated with any player.
    pub fn view_for(&self, tab_id: &TabId) -> Result<GameView, HostError> {
        let player_id = self
            .human_players
            .iter()
            .find_map(|(candidate, player_id)| (candidate == tab_id).then_some(*player_id))
            .ok_or(HostError::NotRoomMember)?;
        Ok(GameEngine::project_view(&self.state, player_id)?)
    }

    /// Applies a human player command.
    ///
    /// # Errors
    ///
    /// Returns an error on revision conflict, turn ownership violation, or core rule rejection.
    pub fn submit_human_command(
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
        Ok(())
    }

    /// Converts a departed human's player seat to bot control.
    pub fn convert_human_to_bot(&mut self, tab_id: &TabId) {
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
    }

    /// Steps the active bot player by computing its command and advancing state.
    ///
    /// # Errors
    ///
    /// Returns an error on revision conflict, match not running, or if a human owns turn.
    pub fn step_bot(&mut self, expected_revision: u64) -> Result<(), HostError> {
        if expected_revision != self.state.revision {
            return Err(HostError::RevisionConflict {
                expected: self.state.revision,
                actual: expected_revision,
            });
        }
        if !matches!(self.state.status, GameStatus::Running) {
            return Err(HostError::MatchNotRunning);
        }
        let actor_id = GameEngine::current_player_id(&self.state)?;
        let actor = self
            .state
            .players
            .iter()
            .find(|player| player.id == actor_id)
            .ok_or(HostError::Core(CoreError::UnknownPlayer(actor_id)))?;
        if actor.kind != PlayerKind::Bot {
            return Err(HostError::BotDoesNotOwnTurn);
        }
        let command = GameEngine::choose_random_bot_command(&mut self.state, actor_id)?;
        GameEngine::apply_command(&mut self.state, actor_id, command)?;
        Ok(())
    }
}
