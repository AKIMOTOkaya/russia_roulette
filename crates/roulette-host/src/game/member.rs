//! Room member entity tracking human and bot participants.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

use roulette_domain::{PlayerId, PlayerKind, RoomMemberId, TabId};

use crate::validation::human_member_id;

/// One participant seat inside a room (either human tab or bot).
#[derive(Debug, Clone)]
pub struct RoomMember {
    /// Room member unique identifier.
    pub id: RoomMemberId,
    /// Participant display name.
    pub name: String,
    /// Human or Bot controller.
    pub kind: PlayerKind,
    /// Browser tab ID if human.
    pub tab_id: Option<TabId>,
    /// Activity timestamp if human.
    pub last_seen: Option<Instant>,
    /// In-game deterministic player ID once match starts.
    pub player_id: Option<PlayerId>,
}

impl RoomMember {
    /// Constructs a human member seat.
    #[must_use]
    pub fn human(tab_id: &TabId, name: &str, now: Instant) -> Self {
        Self {
            id: human_member_id(tab_id),
            name: name.to_owned(),
            kind: PlayerKind::Human,
            tab_id: Some(tab_id.clone()),
            last_seen: Some(now),
            player_id: None,
        }
    }

    /// Constructs a bot member seat.
    #[must_use]
    pub fn bot(id: RoomMemberId, name: String) -> Self {
        Self {
            id,
            name,
            kind: PlayerKind::Bot,
            tab_id: None,
            last_seen: None,
            player_id: None,
        }
    }

    /// Returns tab ID reference if human.
    #[must_use]
    pub const fn tab_id(&self) -> Option<&TabId> {
        self.tab_id.as_ref()
    }

    /// Returns true if this seat belongs to a human player.
    #[must_use]
    pub fn is_human(&self) -> bool {
        self.kind == PlayerKind::Human
    }

    /// Returns true if this seat is controlled by a bot.
    #[must_use]
    pub fn is_bot(&self) -> bool {
        self.kind == PlayerKind::Bot
    }

    /// Updates activity timestamp for a human member.
    pub fn touch(&mut self, now: Instant) {
        if self.is_human() {
            self.last_seen = Some(now);
        }
    }

    /// Checks if a human member has exceeded inactivity timeout.
    #[must_use]
    pub fn is_stale(&self, now: Instant, timeout: Duration) -> bool {
        self.tab_id.is_some()
            && self.last_seen.is_some_and(|last| {
                now.checked_duration_since(last)
                    .is_some_and(|d| d >= timeout)
            })
    }
}
