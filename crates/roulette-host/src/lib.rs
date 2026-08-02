//! In-memory match hosting and local bot orchestration.
//!
//! The MVP host owns one local match, validates revisions, accepts commands for
//! the browser-controlled player, and advances random bots through the same
//! `roulette-core` command path. It contains no HTTP or persistence code.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

use roulette_core::{CoreError, GameEngine};
use roulette_domain::{
    CreateGameConfig, GameState, GameStatus, GameView, PlayerCommand, PlayerId, PlayerKind,
    PlayerSetup,
};

const MIN_BOTS: u8 = 2;
const MAX_BOTS: u8 = 5;
const MAX_AUTOMATED_COMMANDS: usize = 4_096;

/// Inputs for a local browser-controlled match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMatchConfig {
    /// Human display name.
    pub human_name: String,
    /// Number of simple random bots, from two through five.
    pub bot_count: u8,
    /// Explicit deterministic seed.
    pub seed: u64,
}

/// Errors produced by local match orchestration.
#[derive(Debug)]
pub enum HostError {
    /// Bot count lies outside the MVP range.
    InvalidBotCount,
    /// Browser submitted an obsolete or future revision.
    RevisionConflict {
        /// Revision expected by the host.
        expected: u64,
        /// Revision supplied by the browser.
        actual: u64,
    },
    /// Browser attempted to act while a bot owned the turn.
    HumanDoesNotOwnTurn,
    /// Automated play exceeded the safety budget.
    AutomationLimitExceeded,
    /// Deterministic core rejected an operation.
    Core(CoreError),
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBotCount => formatter.write_str("bot count must be between 2 and 5"),
            Self::RevisionConflict { expected, actual } => write!(
                formatter,
                "revision conflict: expected {expected}, received {actual}"
            ),
            Self::HumanDoesNotOwnTurn => {
                formatter.write_str("the human player does not own the turn")
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

/// One in-memory local match controlled by a browser and random bots.
#[derive(Debug)]
pub struct LocalMatch {
    state: GameState,
    human_player_id: PlayerId,
}

impl LocalMatch {
    /// Creates a match with the human first in turn order.
    ///
    /// # Errors
    ///
    /// Returns an error when the bot count or generated Core state is invalid.
    pub fn new(config: &LocalMatchConfig) -> Result<Self, HostError> {
        if !(MIN_BOTS..=MAX_BOTS).contains(&config.bot_count) {
            return Err(HostError::InvalidBotCount);
        }
        let human_name = if config.human_name.trim().is_empty() {
            "玩家".to_owned()
        } else {
            config.human_name.trim().to_owned()
        };
        let mut players = vec![PlayerSetup {
            name: human_name,
            kind: PlayerKind::Human,
        }];
        players.extend((1..=config.bot_count).map(|number| PlayerSetup {
            name: format!("Bot {number}"),
            kind: PlayerKind::Bot,
        }));
        let state = GameEngine::create_game(CreateGameConfig {
            seed: config.seed,
            players,
        })?;
        Ok(Self {
            state,
            human_player_id: PlayerId(1),
        })
    }

    /// Returns the current public snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the authoritative state cannot be projected.
    pub fn view(&self) -> Result<GameView, HostError> {
        Ok(GameEngine::project_view(&self.state, self.human_player_id)?)
    }

    /// Validates and applies a human command, then advances bot turns.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale revision, invalid turn ownership, invalid
    /// Core command, or exhausted automated-play safety budget.
    pub fn submit_human_command(
        &mut self,
        expected_revision: u64,
        command: PlayerCommand,
    ) -> Result<GameView, HostError> {
        if expected_revision != self.state.revision {
            return Err(HostError::RevisionConflict {
                expected: self.state.revision,
                actual: expected_revision,
            });
        }
        if matches!(self.state.status, GameStatus::Running)
            && GameEngine::current_player_id(&self.state)? != self.human_player_id
        {
            return Err(HostError::HumanDoesNotOwnTurn);
        }
        GameEngine::apply_command(&mut self.state, self.human_player_id, command)?;
        self.advance_bots()?;
        self.view()
    }

    /// Returns the authoritative state for replay and tests.
    #[must_use]
    pub const fn state(&self) -> &GameState {
        &self.state
    }

    fn advance_bots(&mut self) -> Result<(), HostError> {
        for _ in 0..MAX_AUTOMATED_COMMANDS {
            if !matches!(self.state.status, GameStatus::Running) {
                return Ok(());
            }
            let actor_id = GameEngine::current_player_id(&self.state)?;
            if actor_id == self.human_player_id {
                return Ok(());
            }
            let command = GameEngine::choose_random_bot_command(&mut self.state, actor_id)?;
            GameEngine::apply_command(&mut self.state, actor_id, command)?;
        }
        Err(HostError::AutomationLimitExceeded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_match(seed: u64) -> LocalMatch {
        LocalMatch::new(&LocalMatchConfig {
            human_name: "Alice".to_owned(),
            bot_count: 2,
            seed,
        })
        .expect("local match")
    }

    #[test]
    fn human_starts_and_bots_return_control() {
        let mut game = local_match(123);
        let initial = game.view().expect("initial view");
        assert_eq!(initial.current_player_id, Some(initial.human_player_id));

        let next = game
            .submit_human_command(initial.revision, PlayerCommand::Wait)
            .expect("human wait");
        assert!(next.revision >= 3);
        assert!(
            !matches!(next.status, GameStatus::Running)
                || next.current_player_id == Some(next.human_player_id)
        );
    }

    #[test]
    fn stale_revision_is_rejected_without_mutation() {
        let mut game = local_match(456);
        let before = game.state().clone();
        let error = game
            .submit_human_command(99, PlayerCommand::Wait)
            .expect_err("stale command must fail");
        assert!(matches!(error, HostError::RevisionConflict { .. }));
        assert_eq!(game.state(), &before);
    }

    #[test]
    fn same_seed_and_human_commands_are_replayable() {
        let mut first = local_match(789);
        let mut second = local_match(789);
        for _ in 0..4 {
            if !matches!(first.state().status, GameStatus::Running) {
                break;
            }
            let first_revision = first.state().revision;
            let second_revision = second.state().revision;
            first
                .submit_human_command(first_revision, PlayerCommand::Wait)
                .expect("first command");
            second
                .submit_human_command(second_revision, PlayerCommand::Wait)
                .expect("second command");
        }
        assert_eq!(first.state(), second.state());
    }
}
