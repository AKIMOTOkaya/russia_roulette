//! Input validation and normalization helpers.

#![forbid(unsafe_code)]

use roulette_domain::{RoomId, RoomMemberId, TabId};

use crate::error::HostError;

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

/// Minimum members required to start a room match.
pub const MIN_MEMBERS: usize = 3;
/// Maximum member capacity of a room.
pub const MAX_MEMBERS: usize = 6;
/// Length of a five-character room code.
pub const ROOM_CODE_LENGTH: usize = 5;
/// Alphabet for generating readable room codes.
pub const ROOM_CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Validates that a browser `TabId` is non-empty and within length bounds.
///
/// # Errors
///
/// Returns an error if the tab ID is empty or exceeds 128 characters.
pub fn validate_tab_id(tab_id: &TabId) -> Result<(), HostError> {
    let value = tab_id.0.trim();
    if value.is_empty() || value.len() > 128 {
        return Err(HostError::InvalidInput("tab id"));
    }
    Ok(())
}

/// Validates that a display name is non-empty and does not exceed character limits.
///
/// # Errors
///
/// Returns an error if the name is empty or exceeds `maximum` characters.
pub fn validate_name(value: &str, maximum: usize, label: &'static str) -> Result<(), HostError> {
    let length = value.trim().chars().count();
    if length == 0 || length > maximum {
        return Err(HostError::InvalidInput(label));
    }
    Ok(())
}

/// Normalizes and validates an optional room password.
///
/// # Errors
///
/// Returns an error if the password is empty or exceeds 32 characters.
pub fn normalize_password(password: Option<&str>) -> Result<Option<String>, HostError> {
    match password {
        None => Ok(None),
        Some(value) if value.is_empty() || value.chars().count() > 32 => {
            Err(HostError::InvalidInput("room password"))
        }
        Some(value) => Ok(Some(value.to_owned())),
    }
}

/// Normalizes and validates a 5-character alphanumeric room identifier.
///
/// # Errors
///
/// Returns an error if the room ID is not 5 alphanumeric characters.
pub fn normalize_room_id(value: &str) -> Result<RoomId, HostError> {
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

/// Generates a standardized human member identifier from a `TabId`.
#[must_use]
pub fn human_member_id(tab_id: &TabId) -> RoomMemberId {
    RoomMemberId(format!("human:{}", tab_id.0))
}

/// Simple deterministic linear congruential / splitmix step for room code generation.
pub fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
