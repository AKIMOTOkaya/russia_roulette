//! Contextual event pools and trigger points.

use roulette_domain::{
    CandidateTrace, Direction, EventId, GameState, PlayerCommand, PlayerId, Position, Terrain,
    Weather,
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
    /// A player is exiting a terrain cell.
    TerrainExited {
        /// Moving player.
        actor_id: PlayerId,
        /// Previous position.
        from: Position,
        /// Landing position.
        to: Position,
        /// Terrain of exiting cell.
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
            Self::ActionIntent { actor_id, .. }
            | Self::TerrainEntered { actor_id, .. }
            | Self::TerrainExited { actor_id, .. } => Some(*actor_id),
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
            Self::TerrainExited { from, .. } => Some(*from),
            Self::ProjectileImpact { position, .. } => Some(*position),
            Self::SecondaryTrigger { position, .. } => *position,
            _ => None,
        }
    }

    /// Human-readable diagnostic description of this trigger.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::RoundStart { round } => format!("大回合开始 (第 {round} 轮)"),
            Self::TurnStart { player_id } => format!("玩家 P{} 回合开始", player_id.0),
            Self::ActionIntent { actor_id, command } => match command {
                PlayerCommand::Move { direction } => {
                    format!("玩家 P{} 意图向 {:?} 移动", actor_id.0, direction)
                }
                PlayerCommand::Shoot { direction } => {
                    format!("玩家 P{} 意图向 {:?} 射击", actor_id.0, direction)
                }
                PlayerCommand::Wait => format!("玩家 P{} 意图等待", actor_id.0),
                PlayerCommand::Suicide => format!("玩家 P{} 意图自裁", actor_id.0),
            },
            Self::TerrainEntered {
                actor_id,
                to,
                terrain,
                ..
            } => {
                format!(
                    "玩家 P{} 踏入 {:?} ({}, {})",
                    actor_id.0, terrain, to.x, to.y
                )
            }
            Self::TerrainExited {
                actor_id,
                from,
                terrain,
                ..
            } => {
                format!(
                    "玩家 P{} 离开 {:?} ({}, {})",
                    actor_id.0, terrain, from.x, from.y
                )
            }
            Self::ProjectileImpact {
                shooter_id,
                position,
                hit_terrain,
                ..
            } => {
                format!(
                    "玩家 P{} 子弹击中 {:?} ({}, {})",
                    shooter_id.0, hit_terrain, position.x, position.y
                )
            }
            Self::SecondaryTrigger {
                tag,
                actor_id,
                position,
            } => {
                use std::fmt::Write;
                let mut s = format!("次生波连锁 [{tag}]");
                if let Some(id) = actor_id {
                    let _ = write!(s, " 涉及 P{}", id.0);
                }
                if let Some(pos) = position {
                    let _ = write!(s, " 坐标 ({}, {})", pos.x, pos.y);
                }
                s
            }
        }
    }
}

/// Scope and severity of a lethal event in terms of player eliminations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LethalScope {
    /// Non-lethal event.
    #[default]
    None,
    /// Strictly eliminates at most a single player (e.g. wall crash, single target shot, mine).
    Single,
    /// Potentially eliminates multiple players (e.g. piercing slug penetrating a line of targets).
    PotentialMulti,
    /// Directly impacts an area or multiple players (catastrophic events).
    Multi,
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
    /// Whether this event can inflict lethal or catastrophic damage/elimination.
    pub is_lethal: bool,
    /// Escalation growth per round without elimination when `is_lethal` is true.
    pub lethality_growth: u32,
    /// Scope of player elimination.
    pub lethal_scope: LethalScope,
    /// Hard probability ceiling permille (e.g. Some(250) for 25.0%), if capped.
    pub max_probability_permille: Option<u32>,
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
            is_lethal: false,
            lethality_growth: 0,
            lethal_scope: LethalScope::None,
            max_probability_permille: None,
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
            is_lethal: false,
            lethality_growth: 0,
            lethal_scope: LethalScope::None,
            max_probability_permille: None,
        }
    }

    /// Creates a single-player lethal pool entry with high escalation and no probability cap.
    #[must_use]
    pub fn lethal(event_id: &'static str, base_weight: u32, lethality_growth: u32) -> Self {
        Self {
            event_id: EventId::new(event_id),
            base_weight,
            is_dampener: false,
            dampening_growth: 0,
            is_lethal: true,
            lethality_growth,
            lethal_scope: LethalScope::Single,
            max_probability_permille: None,
        }
    }

    /// Creates a potentially multi-target lethal pool entry with a maximum probability cap.
    #[must_use]
    pub fn lethal_potential_multi(
        event_id: &'static str,
        base_weight: u32,
        lethality_growth: u32,
        max_permille: u32,
    ) -> Self {
        Self {
            event_id: EventId::new(event_id),
            base_weight,
            is_dampener: false,
            dampening_growth: 0,
            is_lethal: true,
            lethality_growth,
            lethal_scope: LethalScope::PotentialMulti,
            max_probability_permille: Some(max_permille),
        }
    }

    /// Creates a multi-target / catastrophic lethal pool entry with a strict probability cap.
    #[must_use]
    pub fn lethal_multi(
        event_id: &'static str,
        base_weight: u32,
        lethality_growth: u32,
        max_permille: u32,
    ) -> Self {
        Self {
            event_id: EventId::new(event_id),
            base_weight,
            is_dampener: false,
            dampening_growth: 0,
            is_lethal: true,
            lethality_growth,
            lethal_scope: LethalScope::Multi,
            max_probability_permille: Some(max_permille),
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

    /// Selects an event and returns full diagnostic candidate traces, total weight, and roll value.
    #[must_use]
    pub fn roll_with_trace(
        &self,
        depth: u32,
        stalemate_rounds: u32,
        rng_stream: &mut u64,
    ) -> (Option<EventId>, u64, u64, Vec<CandidateTrace>) {
        let depth_u64 = u64::from(depth);
        let stalemate_u64 = u64::from(stalemate_rounds);

        let mut initial_weights = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let eff_weight = if entry.is_dampener {
                u64::from(entry.base_weight)
                    .saturating_add(depth_u64.saturating_mul(u64::from(entry.dampening_growth)))
            } else {
                let base_with_stalemate =
                    u64::from(entry.base_weight).saturating_add(if entry.is_lethal {
                        stalemate_u64.saturating_mul(u64::from(entry.lethality_growth))
                    } else {
                        0
                    });
                (base_with_stalemate / (1 + depth_u64)).max(1)
            };
            initial_weights.push((entry, eff_weight));
        }

        let sum_unrestricted: u64 = initial_weights
            .iter()
            .filter(|(e, _)| e.max_probability_permille.is_none())
            .map(|(_, w)| *w)
            .sum();

        let mut total_weight: u64 = 0;
        let mut weighted_entries = Vec::with_capacity(initial_weights.len());

        for (entry, eff_weight) in initial_weights {
            let final_weight = if let Some(max_permille) = entry.max_probability_permille {
                if max_permille < 1000 && sum_unrestricted > 0 {
                    let cap = (sum_unrestricted.saturating_mul(u64::from(max_permille)))
                        / u64::from(1000 - max_permille);
                    eff_weight.min(cap.max(1))
                } else {
                    eff_weight
                }
            } else {
                eff_weight
            };
            total_weight = total_weight.saturating_add(final_weight);
            weighted_entries.push((entry, final_weight));
        }

        if total_weight == 0 {
            return (None, 0, 0, Vec::new());
        }

        let roll = crate::next_random(rng_stream) % total_weight;
        let mut accumulator: u64 = 0;
        let mut chosen: Option<EventId> = None;

        for (entry, weight) in &weighted_entries {
            accumulator = accumulator.saturating_add(*weight);
            if chosen.is_none() && roll < accumulator {
                chosen = Some(entry.event_id.clone());
            }
        }
        if chosen.is_none() {
            chosen = self.entries.last().map(|e| e.event_id.clone());
        }

        let candidates = weighted_entries
            .into_iter()
            .map(|(entry, eff_weight)| {
                let title = crate::events::catalog::EventRegistry::get(&entry.event_id)
                    .map_or_else(|| entry.event_id.0.clone(), |d| d.title.to_owned());
                let probability_permille = eff_weight
                    .saturating_mul(1000)
                    .checked_div(total_weight)
                    .and_then(|v| u32::try_from(v).ok())
                    .unwrap_or(0);
                let selected = chosen.as_ref() == Some(&entry.event_id);
                CandidateTrace {
                    event_id: entry.event_id.clone(),
                    title,
                    base_weight: entry.base_weight,
                    effective_weight: eff_weight,
                    probability_permille,
                    is_dampener: entry.is_dampener,
                    is_lethal: entry.is_lethal,
                    selected,
                }
            })
            .collect();

        (chosen, total_weight, roll, candidates)
    }

    /// Selects an event from the pool using branch depth, stalemate rounds, and events random stream.
    #[must_use]
    pub fn roll(&self, depth: u32, stalemate_rounds: u32, rng_stream: &mut u64) -> Option<EventId> {
        self.roll_with_trace(depth, stalemate_rounds, rng_stream).0
    }
}

/// Resolves the contextual event pool matching a given trigger point and game state.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn resolve_pool(trigger: &TriggerPoint, state: &GameState) -> Option<EventPool> {
    match trigger {
        TriggerPoint::RoundStart { round } => {
            if *round >= 2 {
                match state.weather {
                    Weather::Clear => Some(EventPool::new(
                        "ambient_round_weather_clear",
                        vec![
                            PoolEntry::lethal_multi("evt_meteor_strike", 6, 25, 120),
                            PoolEntry::normal("evt_weather_blizzard", 25),
                            PoolEntry::normal("evt_weather_heatwave", 20),
                            PoolEntry::dampener("evt_nothing_happens", 10, 45),
                        ],
                    )),
                    Weather::Blizzard => Some(EventPool::new(
                        "ambient_round_weather_blizzard",
                        vec![
                            PoolEntry::lethal_multi("evt_meteor_strike", 6, 25, 120),
                            PoolEntry::normal("evt_weather_heatwave", 35),
                            PoolEntry::dampener("evt_nothing_happens", 10, 45),
                        ],
                    )),
                    Weather::Heatwave => Some(EventPool::new(
                        "ambient_round_weather_heatwave",
                        vec![
                            PoolEntry::lethal_multi("evt_meteor_strike", 6, 25, 120),
                            PoolEntry::normal("evt_weather_blizzard", 30),
                            PoolEntry::dampener("evt_nothing_happens", 10, 45),
                        ],
                    )),
                    Weather::DenseFog => Some(EventPool::new(
                        "ambient_round_weather_fog",
                        vec![
                            PoolEntry::lethal_multi("evt_meteor_strike", 6, 25, 120),
                            PoolEntry::normal("evt_weather_blizzard", 25),
                            PoolEntry::dampener("evt_nothing_happens", 10, 45),
                        ],
                    )),
                }
            } else {
                None
            }
        }
        TriggerPoint::ActionIntent { actor_id, command } => match command {
            PlayerCommand::Shoot { .. } => Some(EventPool::new(
                "shoot_intent",
                vec![
                    PoolEntry::normal("evt_recoil_knockback", 20),
                    PoolEntry::normal("evt_disoriented_reverse_shot", 15),
                    PoolEntry::lethal_potential_multi("evt_piercing_slug", 20, 20, 250),
                    PoolEntry::lethal("evt_ricochet_deadly", 15, 45),
                    PoolEntry::dampener("evt_revolver_misfire", 15, 25),
                    PoolEntry::dampener("evt_nothing_happens", 15, 45),
                ],
            )),
            PlayerCommand::Move { direction } => {
                let is_wall_or_boundary = crate::player_index(state, *actor_id)
                    .ok()
                    .and_then(|idx| {
                        let pos = state.players[idx].position?;
                        let target = crate::step_position(pos, *direction, state.map_size);
                        Some(target.is_none_or(|t| {
                            let t_idx = crate::map_index(t, state.map_size).unwrap_or(0);
                            state.terrain[t_idx] == Terrain::Wall
                        }))
                    })
                    .unwrap_or(false);

                if is_wall_or_boundary {
                    Some(EventPool::new(
                        "wall_crash",
                        vec![
                            PoolEntry::lethal("evt_wall_fatal_concussion", 45, 20),
                            PoolEntry::lethal("evt_wall_collapse_crush", 35, 15),
                            PoolEntry::lethal_potential_multi(
                                "evt_wall_rebound_detonation",
                                20,
                                10,
                                250,
                            ),
                            PoolEntry::dampener("evt_wall_stun_survive", 25, 40),
                            PoolEntry::normal("evt_wall_breakthrough", 5),
                        ],
                    ))
                } else {
                    Some(EventPool::new(
                        "move_intent",
                        vec![
                            PoolEntry::normal("evt_sprint_dash", 15),
                            PoolEntry::normal("evt_spatial_swap", 7),
                            PoolEntry::normal("evt_stumble_trip", 3),
                            PoolEntry::dampener("evt_nothing_happens", 75, 45),
                        ],
                    ))
                }
            }
            _ => None,
        },
        TriggerPoint::TerrainEntered { terrain, .. } => match terrain {
            Terrain::Empty => Some(EventPool::new(
                "terrain_enter_empty",
                vec![
                    PoolEntry::normal("evt_pebble_kick", 1),
                    PoolEntry::dampener("evt_nothing_happens", 99, 50),
                ],
            )),
            Terrain::Ice => Some(EventPool::new(
                "terrain_enter_ice",
                vec![
                    PoolEntry::normal("evt_ice_slide", 45),
                    PoolEntry::lethal("evt_ice_crack_collapse", 25, 15),
                    PoolEntry::dampener("evt_nothing_happens", 30, 40),
                ],
            )),
            Terrain::Water => Some(EventPool::new(
                "terrain_enter_water",
                vec![
                    PoolEntry::normal("evt_water_bogged_down", 50),
                    PoolEntry::normal("evt_water_splash", 20),
                    PoolEntry::dampener("evt_nothing_happens", 30, 40),
                ],
            )),
            Terrain::HighGround => Some(EventPool::new(
                "terrain_enter_high_ground",
                vec![
                    PoolEntry::normal("evt_high_ground_vantage", 40),
                    PoolEntry::normal("evt_rockslide_tumble", 20),
                    PoolEntry::dampener("evt_nothing_happens", 40, 40),
                ],
            )),
            Terrain::Mine => Some(EventPool::new(
                "terrain_enter_mine",
                vec![
                    PoolEntry::lethal("evt_mine_detonation", 85, 20),
                    PoolEntry::normal("evt_mine_dud", 10),
                    PoolEntry::dampener("evt_nothing_happens", 5, 40),
                ],
            )),
            Terrain::Medkit => Some(EventPool::new(
                "terrain_enter_medkit",
                vec![
                    PoolEntry::normal("evt_medkit_collect", 95),
                    PoolEntry::dampener("evt_nothing_happens", 5, 40),
                ],
            )),
            _ => Some(EventPool::new(
                "terrain_enter_neutral",
                vec![PoolEntry::dampener("evt_nothing_happens", 100, 50)],
            )),
        },
        TriggerPoint::TerrainExited { terrain, .. } => match terrain {
            Terrain::Empty => Some(EventPool::new(
                "terrain_exit_empty",
                vec![PoolEntry::dampener("evt_nothing_happens", 100, 50)],
            )),
            Terrain::Ice => Some(EventPool::new(
                "terrain_exit_ice",
                vec![
                    PoolEntry::normal("evt_ice_exit_crack", 30),
                    PoolEntry::dampener("evt_nothing_happens", 70, 50),
                ],
            )),
            Terrain::Water => Some(EventPool::new(
                "terrain_exit_water",
                vec![
                    PoolEntry::normal("evt_water_exit_surge", 30),
                    PoolEntry::dampener("evt_nothing_happens", 70, 50),
                ],
            )),
            Terrain::HighGround => Some(EventPool::new(
                "terrain_exit_high_ground",
                vec![
                    PoolEntry::normal("evt_high_ground_leap", 30),
                    PoolEntry::dampener("evt_nothing_happens", 70, 50),
                ],
            )),
            _ => Some(EventPool::new(
                "terrain_exit_neutral",
                vec![PoolEntry::dampener("evt_nothing_happens", 100, 50)],
            )),
        },
        TriggerPoint::ProjectileImpact {
            hit_terrain: Terrain::Crate,
            ..
        } => Some(EventPool::new(
            "crate_impact",
            vec![
                PoolEntry::lethal("evt_crate_surprise_mine", 25, 20),
                PoolEntry::normal("evt_crate_splinter_blast", 30),
                PoolEntry::normal("evt_crate_surprise_medkit", 20),
                PoolEntry::dampener("evt_dust_settles", 10, 40),
            ],
        )),
        TriggerPoint::SecondaryTrigger { tag, .. } => match tag.as_str() {
            "freeze_waters" => Some(EventPool::new(
                "freeze_waters",
                vec![PoolEntry::dampener("evt_water_freeze_ice", 100, 0)],
            )),
            "melt_ices" => Some(EventPool::new(
                "melt_ices",
                vec![PoolEntry::dampener("evt_weather_heatwave", 100, 0)],
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
