//! Contextual event pools and trigger points.

use roulette_domain::{
    Direction, EventId, GameState, PlayerCommand, PlayerId, Position, Terrain, Weather,
};

/// Trigger point providing contextual data to the event system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerPoint {
    /// Big round advanced (round counter incremented).
    RoundStart {
        /// New round number.
        round: u32,
    },
    /// A player is about to start their turn.
    TurnStart {
        /// Acting player.
        player_id: PlayerId,
    },
    /// An actor is about to execute an action.
    ActionIntent {
        /// Intending actor.
        actor_id: PlayerId,
        /// Submitted command.
        command: PlayerCommand,
    },
    /// A player stepped onto a terrain cell.
    TerrainEntered {
        /// Moving player.
        actor_id: PlayerId,
        /// Previous position.
        from: Position,
        /// Landing position.
        to: Position,
        /// Terrain of landing cell.
        terrain: Terrain,
        /// Movement direction.
        direction: Direction,
    },
    /// A shot bullet collided with an obstacle, water, or player.
    ProjectileImpact {
        /// Shooting player.
        shooter_id: PlayerId,
        /// Coordinate of impact.
        position: Position,
        /// Terrain at impact point.
        hit_terrain: Terrain,
        /// Target player at impact point, if any.
        hit_player: Option<PlayerId>,
    },
    /// A secondary trigger emitted by an event effect.
    SecondaryTrigger {
        /// Semantic tag categorizing the secondary wave.
        tag: String,
        /// Relevant actor, if any.
        actor_id: Option<PlayerId>,
        /// Relevant map coordinate, if any.
        position: Option<Position>,
    },
}

impl TriggerPoint {
    /// Returns the primary actor associated with this trigger, if applicable.
    #[must_use]
    pub fn actor_id(&self) -> Option<PlayerId> {
        match self {
            Self::TurnStart { player_id } => Some(*player_id),
            Self::ActionIntent { actor_id, .. } | Self::TerrainEntered { actor_id, .. } => {
                Some(*actor_id)
            }
            Self::ProjectileImpact { shooter_id, .. } => Some(*shooter_id),
            Self::SecondaryTrigger { actor_id, .. } => *actor_id,
            Self::RoundStart { .. } => None,
        }
    }

    /// Returns the map position associated with this trigger, if applicable.
    #[must_use]
    pub fn position(&self) -> Option<Position> {
        match self {
            Self::TerrainEntered { to, .. } => Some(*to),
            Self::ProjectileImpact { position, .. } => Some(*position),
            Self::SecondaryTrigger { position, .. } => *position,
            _ => None,
        }
    }
}

/// One candidate entry within an event pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolEntry {
    /// Event ID chosen when this entry is rolled.
    pub event_id: EventId,
    /// Base selection weight before dampening adjustments.
    pub base_weight: u32,
    /// Whether this event is a chain dampener that closes out the reaction.
    pub is_dampener: bool,
    /// Growth rate of weight per unit of branch path depth when `is_dampener` is true.
    pub dampening_growth: u32,
}

impl PoolEntry {
    /// Creates a normal (divergent/propagating) pool entry.
    #[must_use]
    pub fn normal(event_id: &'static str, base_weight: u32) -> Self {
        Self {
            event_id: EventId::new(event_id),
            base_weight,
            is_dampener: false,
            dampening_growth: 0,
        }
    }

    /// Creates a dampening (convergent/closing) pool entry.
    #[must_use]
    pub fn dampener(event_id: &'static str, base_weight: u32, dampening_growth: u32) -> Self {
        Self {
            event_id: EventId::new(event_id),
            base_weight,
            is_dampener: true,
            dampening_growth,
        }
    }
}

/// Contextual collection of events with dynamic weight dampening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPool {
    /// Diagnostic pool name.
    pub name: &'static str,
    /// Candidate event entries.
    pub entries: Vec<PoolEntry>,
}

impl EventPool {
    /// Creates a new named event pool.
    #[must_use]
    pub fn new(name: &'static str, entries: Vec<PoolEntry>) -> Self {
        Self { name, entries }
    }

    /// Selects an event from the pool using the branch path depth and the events random stream.
    #[must_use]
    pub fn roll(&self, depth: u32, rng_stream: &mut u64) -> Option<EventId> {
        let mut total_weight: u64 = 0;
        let mut weighted_entries = Vec::with_capacity(self.entries.len());

        for entry in &self.entries {
            let eff_weight = if entry.is_dampener {
                u64::from(
                    entry
                        .base_weight
                        .saturating_add(depth.saturating_mul(entry.dampening_growth)),
                )
            } else {
                let divisor = u64::from(1 + depth);
                (u64::from(entry.base_weight) / divisor).max(1)
            };
            total_weight = total_weight.saturating_add(eff_weight);
            weighted_entries.push((entry, eff_weight));
        }

        if total_weight == 0 {
            return None;
        }

        let roll = crate::next_random(rng_stream) % total_weight;
        let mut accumulator: u64 = 0;
        for (entry, weight) in weighted_entries {
            accumulator = accumulator.saturating_add(weight);
            if roll < accumulator {
                return Some(entry.event_id.clone());
            }
        }

        self.entries.last().map(|e| e.event_id.clone())
    }
}

/// Resolves the contextual event pool matching a given trigger point and game state.
#[must_use]
pub fn resolve_pool(trigger: &TriggerPoint, state: &GameState) -> Option<EventPool> {
    match trigger {
        TriggerPoint::RoundStart { round } => {
            // After round 1, ambient weather shifts may occur if currently Clear
            if *round >= 2 && state.weather == Weather::Clear {
                Some(EventPool::new(
                    "ambient_round_weather",
                    vec![
                        PoolEntry::normal("evt_weather_blizzard", 25),
                        PoolEntry::dampener("evt_nothing_happens", 75, 40),
                    ],
                ))
            } else {
                None
            }
        }
        TriggerPoint::ActionIntent {
            command: PlayerCommand::Shoot { .. },
            ..
        } => Some(EventPool::new(
            "shoot_intent",
            vec![
                PoolEntry::normal("evt_disoriented_reverse_shot", 15),
                PoolEntry::dampener("evt_nothing_happens", 85, 40),
            ],
        )),
        TriggerPoint::TerrainEntered {
            terrain: Terrain::Ice,
            ..
        } => Some(EventPool::new(
            "ice_terrain_impact",
            vec![
                PoolEntry::normal("evt_ice_slide", 70),
                PoolEntry::dampener("evt_dust_settles", 30, 40),
            ],
        )),
        TriggerPoint::ProjectileImpact {
            hit_terrain: Terrain::Crate,
            ..
        } => Some(EventPool::new(
            "crate_impact",
            vec![
                PoolEntry::normal("evt_crate_splinter_blast", 70),
                PoolEntry::dampener("evt_dust_settles", 30, 40),
            ],
        )),
        TriggerPoint::SecondaryTrigger { tag, .. } => match tag.as_str() {
            "freeze_waters" => Some(EventPool::new(
                "freeze_waters",
                vec![
                    // Guaranteed deterministic response (no nothing-happens)
                    PoolEntry::dampener("evt_water_freeze_ice", 100, 0),
                ],
            )),
            "crate_blast" => Some(EventPool::new(
                "splinter_shockwave",
                vec![
                    PoolEntry::normal("evt_splinter_scratch", 40),
                    PoolEntry::dampener("evt_dust_settles", 60, 50),
                ],
            )),
            _ => None,
        },
        _ => None,
    }
}
