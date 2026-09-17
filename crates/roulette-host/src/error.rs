//! Error definitions for host, service, and repository layers.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

use roulette_core::CoreError;

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
    /// Automated step requested while a human owned the turn.
    BotDoesNotOwnTurn,
    /// Automated play exceeded the safety budget.
    AutomationLimitExceeded,
    /// Deterministic Core rejected an operation.
    Core(CoreError),
    /// Repository failure during host operation.
    Repository(RepositoryError),
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
            Self::BotDoesNotOwnTurn => {
                formatter.write_str("a bot does not own the current player turn")
            }
            Self::AutomationLimitExceeded => {
                formatter.write_str("automated bot play exceeded its safety limit")
            }
            Self::Core(error) => Display::fmt(error, formatter),
            Self::Repository(error) => write!(formatter, "repository error: {error}"),
        }
    }
}

impl Error for HostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::Repository(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CoreError> for HostError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<RepositoryError> for HostError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

/// Errors produced by persistence operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    /// Target entity was not found in repository.
    NotFound,
    /// Storage failure or lock corruption.
    Storage(String),
    /// Conflict on unique key or concurrent update.
    Conflict(String),
}

impl Display for RepositoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("entity not found in repository"),
            Self::Storage(message) => write!(formatter, "storage failure: {message}"),
            Self::Conflict(message) => write!(formatter, "storage conflict: {message}"),
        }
    }
}

impl Error for RepositoryError {}

/// High-level application service errors suitable for cross-interface translation (HTTP, MCP, CLI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// Client input validation or malformed argument.
    BadRequest(String),
    /// Entity or resource not found.
    NotFound(String),
    /// Permission denied, wrong password, or unauthorized.
    Forbidden(String),
    /// State conflict, revision mismatch, or invalid phase.
    Conflict(String),
    /// Internal failure, unexpected exception, or engine bug.
    Internal(String),
}

impl Display for ServiceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(msg) => write!(formatter, "bad request: {msg}"),
            Self::NotFound(msg) => write!(formatter, "not found: {msg}"),
            Self::Forbidden(msg) => write!(formatter, "forbidden: {msg}"),
            Self::Conflict(msg) => write!(formatter, "conflict: {msg}"),
            Self::Internal(msg) => write!(formatter, "internal error: {msg}"),
        }
    }
}

impl Error for ServiceError {}

impl From<HostError> for ServiceError {
    fn from(error: HostError) -> Self {
        match error {
            HostError::RoomNotFound => Self::NotFound("room not found".to_string()),
            HostError::NotRoomMember => Self::Forbidden("tab is not a room member".to_string()),
            HostError::IncorrectRoomPassword => {
                Self::Forbidden("incorrect room password".to_string())
            }
            HostError::OwnerRequired => {
                Self::Forbidden("room owner permission required".to_string())
            }
            HostError::HumanDoesNotOwnTurn => {
                Self::Forbidden("this tab does not own the current player turn".to_string())
            }
            HostError::InvalidInput(msg) => Self::BadRequest(format!("invalid input: {msg}")),
            HostError::InvalidMemberCount => {
                Self::BadRequest("a match requires 3 to 6 room members".to_string())
            }
            HostError::AlreadyInRoom => Self::Conflict("tab already belongs to a room".to_string()),
            HostError::RoomFull => Self::Conflict("room is full".to_string()),
            HostError::RoomNotWaiting => {
                Self::Conflict("room is not waiting for players".to_string())
            }
            HostError::InvalidMember => Self::Conflict("invalid room member".to_string()),
            HostError::MatchNotRunning => Self::Conflict("match is not running".to_string()),
            HostError::RevisionConflict { expected, actual } => Self::Conflict(format!(
                "revision conflict: expected {expected}, received {actual}"
            )),
            HostError::BotDoesNotOwnTurn => {
                Self::Conflict("a bot does not own the current player turn".to_string())
            }
            HostError::AutomationLimitExceeded => {
                Self::Internal("automated bot play exceeded safety limit".to_string())
            }
            HostError::Core(err) => Self::Internal(err.to_string()),
            HostError::Repository(err) => match err {
                RepositoryError::NotFound => Self::NotFound("room not found".to_string()),
                RepositoryError::Conflict(msg) => Self::Conflict(msg),
                RepositoryError::Storage(msg) => Self::Internal(msg),
            },
        }
    }
}

impl From<RepositoryError> for ServiceError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound => Self::NotFound("resource not found".to_string()),
            RepositoryError::Conflict(msg) => Self::Conflict(msg),
            RepositoryError::Storage(msg) => Self::Internal(msg),
        }
    }
}
