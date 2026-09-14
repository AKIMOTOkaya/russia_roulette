//! Deterministic rules engine for Russian Roulette.
//!
//! The engine owns command validation, map generation, state transitions,
//! explicit random streams, structured events, and public-view projection. It
//! does not access networks, databases, files, clocks, or language models.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

use roulette_domain::{
    BlockReason, CellView, CreateGameConfig, Direction, EliminationCause, GameEvent,
    GameNotification, GameRecord, GameRecordContent, GameState, GameStatus, GameView,
    NotificationLevel, PlayerCommand, PlayerId, PlayerKind, PlayerState, PlayerStatus, Position,
    RngStreams, Terrain, TerrainLayer, Weather,
};

pub mod events;

const MIN_PLAYERS: usize = 3;
const MAX_PLAYERS: usize = 6;
const BASE_SHOT_RANGE: u8 = 5;
const RECORD_VIEW_LIMIT: usize = 60;

/// Errors returned when a state or command violates the rules contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Game creation requires three to six players for the MVP.
    InvalidPlayerCount,
    /// Every player requires a non-empty display name.
    EmptyPlayerName,
    /// A command referenced a player not present in the state.
    UnknownPlayer(PlayerId),
    /// A command was submitted after the match finished.
    GameAlreadyFinished,
    /// The actor is alive but is not the current turn owner.
    NotCurrentTurn {
        /// Player expected to act.
        expected: PlayerId,
        /// Player supplied with the command.
        actual: PlayerId,
    },
    /// Eliminated players cannot act.
    PlayerEliminated(PlayerId),
    /// The authoritative state is internally inconsistent.
    InvalidState(&'static str),
}

impl Display for CoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlayerCount => formatter.write_str("MVP matches require 3 to 6 players"),
            Self::EmptyPlayerName => formatter.write_str("player names cannot be empty"),
            Self::UnknownPlayer(player_id) => write!(formatter, "unknown player {}", player_id.0),
            Self::GameAlreadyFinished => formatter.write_str("the game has already finished"),
            Self::NotCurrentTurn { expected, actual } => write!(
                formatter,
                "player {} cannot act; player {} owns the turn",
                actual.0, expected.0
            ),
            Self::PlayerEliminated(player_id) => {
                write!(formatter, "player {} has been eliminated", player_id.0)
            }
            Self::InvalidState(message) => write!(formatter, "invalid game state: {message}"),
        }
    }
}

impl Error for CoreError {}

/// Stateless entry point for deterministic game operations.
#[derive(Debug, Default, Clone, Copy)]
pub struct GameEngine;

impl GameEngine {
    /// Creates a complete game from explicit players and seed.
    ///
    /// # Errors
    ///
    /// Returns an error when the player list is invalid or a consistent map
    /// and initial state cannot be constructed.
    pub fn create_game(config: CreateGameConfig) -> Result<GameState, CoreError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&config.players.len()) {
            return Err(CoreError::InvalidPlayerCount);
        }
        if config
            .players
            .iter()
            .any(|player| player.name.trim().is_empty())
        {
            return Err(CoreError::EmptyPlayerName);
        }

        let map_size = u8::try_from((config.players.len() + 2).max(5))
            .map_err(|_| CoreError::InvalidState("map size does not fit in u8"))?;
        let cell_count = usize::from(map_size) * usize::from(map_size);
        let mut rng = RngStreams {
            map: derive_stream(config.seed, 0x4D41_5001),
            encounter: derive_stream(config.seed, 0x454E_4302),
            combat: derive_stream(config.seed, 0x434F_4D03),
            bot: derive_stream(config.seed, 0x424F_5404),
            events: derive_stream(config.seed, 0x4556_5405),
        };
        let mut terrain = vec![Terrain::Empty; cell_count];

        let wall_count = config.players.len().div_ceil(3);
        for _ in 0..wall_count {
            place_terrain(&mut terrain, Terrain::Wall, &mut rng.map)?;
        }
        let special_terrain = [
            Terrain::Water,
            Terrain::Crate,
            Terrain::Mine,
            Terrain::Medkit,
            Terrain::HighGround,
        ];
        for special in special_terrain.into_iter().take(config.players.len()) {
            place_terrain(&mut terrain, special, &mut rng.map)?;
        }

        let mut players = Vec::with_capacity(config.players.len());
        for (index, setup) in config.players.into_iter().enumerate() {
            let position_index = choose_empty_cell(&terrain, &players, map_size, &mut rng.map)?;
            let player_number = u32::try_from(index + 1)
                .map_err(|_| CoreError::InvalidState("player id does not fit in u32"))?;
            players.push(PlayerState {
                id: PlayerId(player_number),
                name: setup.name.trim().to_owned(),
                kind: setup.kind,
                status: PlayerStatus::Alive,
                position: Some(position_from_index(position_index, map_size)?),
                has_shield: false,
                water_turns: 0,
                consecutive_shots: 0,
            });
        }

        let turn_order = players.iter().map(|player| player.id).collect::<Vec<_>>();
        let first_player = turn_order[0];
        Ok(GameState {
            seed: config.seed,
            revision: 0,
            map_size,
            terrain,
            players,
            turn_order,
            current_turn_index: 0,
            round: 1,
            status: GameStatus::Running,
            rng,
            weather: Weather::Clear,
            next_record_sequence: 3,
            records: vec![
                GameRecord {
                    sequence: 1,
                    content: GameRecordContent::Notification {
                        level: NotificationLevel::Info,
                        notification: GameNotification::MatchStarted { seed: config.seed },
                    },
                },
                GameRecord {
                    sequence: 2,
                    content: GameRecordContent::Turn {
                        player_id: first_player,
                        round: 1,
                    },
                },
            ],
            event_traces: Vec::new(),
            rounds_without_elimination: 0,
        })
    }

    /// Returns the player currently expected to act.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is finished or its turn index is invalid.
    pub fn current_player_id(state: &GameState) -> Result<PlayerId, CoreError> {
        if !matches!(state.status, GameStatus::Running) {
            return Err(CoreError::GameAlreadyFinished);
        }
        state
            .turn_order
            .get(state.current_turn_index)
            .copied()
            .ok_or(CoreError::InvalidState(
                "current turn index is out of range",
            ))
    }

    /// Applies one validated command and returns the log records it produced.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is finished, the actor does not own the
    /// turn, the actor is eliminated, or the state violates a Core invariant.
    pub fn apply_command(
        state: &mut GameState,
        actor_id: PlayerId,
        command: PlayerCommand,
    ) -> Result<Vec<GameRecord>, CoreError> {
        let expected = Self::current_player_id(state)?;
        if expected != actor_id {
            return Err(CoreError::NotCurrentTurn {
                expected,
                actual: actor_id,
            });
        }
        let actor_index = player_index(state, actor_id)?;
        if state.players[actor_index].status != PlayerStatus::Alive {
            return Err(CoreError::PlayerEliminated(actor_id));
        }

        let record_start = state.records.len();
        append_record(state, GameRecordContent::Action { actor_id, command })?;

        let intent_trigger = events::TriggerPoint::ActionIntent { actor_id, command };
        let outcome = events::EventTreePipeline::default().run(state, intent_trigger)?;
        let effective_command = outcome.override_command.unwrap_or(command);

        if !outcome.action_canceled {
            match effective_command {
                PlayerCommand::Move { direction } => {
                    state.players[actor_index].consecutive_shots = 0;
                    apply_move(state, actor_index, direction)?;
                    if outcome.sprint_dash
                        && state.players[actor_index].status == PlayerStatus::Alive
                    {
                        apply_move(state, actor_index, direction)?;
                    }
                }
                PlayerCommand::Shoot { direction } => {
                    apply_shot(
                        state,
                        actor_index,
                        direction,
                        outcome.piercing_shot,
                        outcome.deadly_ricochet,
                    )?;
                    if let Some(recoil_dir) = outcome.recoil_direction {
                        apply_recoil(state, actor_index, recoil_dir)?;
                    }
                }
                PlayerCommand::Wait => {
                    state.players[actor_index].consecutive_shots = 0;
                }
                PlayerCommand::Suicide => {
                    state.players[actor_index].consecutive_shots = 0;
                    eliminate_player(state, actor_index, EliminationCause::Suicide, None, false)?;
                }
            }
        }

        resolve_water(state, actor_index)?;
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(CoreError::InvalidState("revision overflow"))?;
        finish_or_advance_turn(state)?;
        Ok(state.records[record_start..].to_vec())
    }

    /// Chooses a simple deterministic random command for the current bot.
    ///
    /// # Errors
    ///
    /// Returns an error when the actor does not own the turn, is not a bot, or
    /// the state violates a Core invariant.
    pub fn choose_random_bot_command(
        state: &mut GameState,
        actor_id: PlayerId,
    ) -> Result<PlayerCommand, CoreError> {
        let expected = Self::current_player_id(state)?;
        if expected != actor_id {
            return Err(CoreError::NotCurrentTurn {
                expected,
                actual: actor_id,
            });
        }
        let actor_index = player_index(state, actor_id)?;
        if state.players[actor_index].kind != PlayerKind::Bot {
            return Err(CoreError::InvalidState(
                "random policy requested for a human",
            ));
        }

        let action_roll = random_index(&mut state.rng.bot, 10)?;
        let direction = match random_index(&mut state.rng.bot, 4)? {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        };
        Ok(match action_roll {
            0..=3 => PlayerCommand::Move { direction },
            4..=7 => PlayerCommand::Shoot { direction },
            _ => PlayerCommand::Wait,
        })
    }

    /// Projects the current full-visibility MVP state for the local browser.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested human player is unknown or the
    /// authoritative map and turn data are inconsistent.
    pub fn project_view(
        state: &GameState,
        human_player_id: PlayerId,
    ) -> Result<GameView, CoreError> {
        player_index(state, human_player_id)?;
        let cells = state
            .terrain
            .iter()
            .copied()
            .enumerate()
            .map(|(index, terrain)| {
                let position = position_from_index(index, state.map_size)?;
                let player_id = state
                    .players
                    .iter()
                    .find(|player| {
                        player.status == PlayerStatus::Alive && player.position == Some(position)
                    })
                    .map(|player| player.id);
                Ok(CellView {
                    position,
                    terrain,
                    player_id,
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let record_start = state.records.len().saturating_sub(RECORD_VIEW_LIMIT);
        let current_player_id = if matches!(state.status, GameStatus::Running) {
            Some(Self::current_player_id(state)?)
        } else {
            None
        };

        Ok(GameView {
            seed: state.seed,
            revision: state.revision,
            map_size: state.map_size,
            cells,
            players: state.players.clone(),
            current_player_id,
            human_player_id,
            round: state.round,
            status: state.status,
            weather: state.weather,
            records: state.records[record_start..].to_vec(),
            event_traces: state.event_traces.clone(),
            rounds_without_elimination: state.rounds_without_elimination,
        })
    }
}

fn apply_move(
    state: &mut GameState,
    actor_index: usize,
    direction: Direction,
) -> Result<(), CoreError> {
    let actor_id = state.players[actor_index].id;
    let from = state.players[actor_index]
        .position
        .ok_or(CoreError::InvalidState("living actor has no position"))?;
    let Some(target) = step_position(from, direction, state.map_size) else {
        push_event(
            state,
            GameEvent::MoveBlocked {
                actor_id,
                reason: BlockReason::Boundary,
            },
        )?;
        return Ok(());
    };
    let target_index = map_index(target, state.map_size)?;
    match state.terrain[target_index] {
        Terrain::Wall => {
            push_event(
                state,
                GameEvent::MoveBlocked {
                    actor_id,
                    reason: BlockReason::Wall,
                },
            )?;
            return Ok(());
        }
        Terrain::Crate => {
            push_event(
                state,
                GameEvent::MoveBlocked {
                    actor_id,
                    reason: BlockReason::Crate,
                },
            )?;
            return Ok(());
        }
        _ => {}
    }

    if let Some(defender_index) = living_player_index_at(state, target, Some(actor_id)) {
        let defender_id = state.players[defender_index].id;
        let attacker_wins = random_index(&mut state.rng.encounter, 2)? == 0;
        let winner_id = if attacker_wins { actor_id } else { defender_id };
        push_event(
            state,
            GameEvent::ElbowDuel {
                attacker_id: actor_id,
                defender_id,
                winner_id,
            },
        )?;
        if attacker_wins {
            eliminate_player(
                state,
                defender_index,
                EliminationCause::ElbowDuel,
                Some(actor_id),
                false,
            )?;
            complete_move_step(state, actor_index, from, target, direction)?;
        } else {
            eliminate_player(
                state,
                actor_index,
                EliminationCause::ElbowDuel,
                Some(defender_id),
                false,
            )?;
        }
        return Ok(());
    }

    complete_move_step(state, actor_index, from, target, direction)
}

fn complete_move_step(
    state: &mut GameState,
    actor_index: usize,
    from: Position,
    to: Position,
    direction: Direction,
) -> Result<(), CoreError> {
    let actor_id = state.players[actor_index].id;
    let from_terrain = state.terrain[map_index(from, state.map_size)?];

    // 1. TerrainExited check
    let exit_trigger = events::TriggerPoint::TerrainExited {
        actor_id,
        from,
        to,
        terrain: from_terrain,
        direction,
    };
    let exit_outcome = events::EventTreePipeline::default().run(state, exit_trigger)?;
    if state.players[actor_index].status != PlayerStatus::Alive || exit_outcome.action_canceled {
        return Ok(());
    }

    // 2. Position update and moved event
    state.players[actor_index].position = Some(to);
    push_event(state, GameEvent::Moved { actor_id, from, to })?;

    // 3. TerrainEntered check
    let to_terrain = state.terrain[map_index(to, state.map_size)?];
    let enter_trigger = events::TriggerPoint::TerrainEntered {
        actor_id,
        from,
        to,
        terrain: to_terrain,
        direction,
    };
    events::EventTreePipeline::default().run(state, enter_trigger)?;

    Ok(())
}

pub(crate) fn resolve_landing_terrain(
    state: &mut GameState,
    actor_index: usize,
    position: Position,
) -> Result<(), CoreError> {
    let terrain_index = map_index(position, state.map_size)?;
    match state.terrain[terrain_index] {
        Terrain::Mine => {
            state.terrain[terrain_index] = Terrain::Empty;
            push_event(
                state,
                GameEvent::TerrainChanged {
                    position,
                    from: Terrain::Mine,
                    to: Terrain::Empty,
                },
            )?;
            eliminate_player(state, actor_index, EliminationCause::Mine, None, true)?;
        }
        Terrain::Medkit => {
            state.players[actor_index].has_shield = true;
            state.terrain[terrain_index] = Terrain::Empty;
            push_event(
                state,
                GameEvent::ItemCollected {
                    player_id: state.players[actor_index].id,
                    item: Terrain::Medkit,
                },
            )?;
            push_event(
                state,
                GameEvent::TerrainChanged {
                    position,
                    from: Terrain::Medkit,
                    to: Terrain::Empty,
                },
            )?;
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn apply_shot(
    state: &mut GameState,
    actor_index: usize,
    direction: Direction,
    piercing: bool,
    ricochet: bool,
) -> Result<(), CoreError> {
    let actor_id = state.players[actor_index].id;
    state.players[actor_index].consecutive_shots = state.players[actor_index]
        .consecutive_shots
        .saturating_add(1);
    if state.players[actor_index].consecutive_shots >= 3 {
        state.players[actor_index].consecutive_shots = 0;
        push_event(
            state,
            GameEvent::EmptyChamber {
                actor_id,
                forced: true,
            },
        )?;
        return Ok(());
    }
    if random_index(&mut state.rng.combat, 4)? == 0 {
        push_event(
            state,
            GameEvent::EmptyChamber {
                actor_id,
                forced: false,
            },
        )?;
        return Ok(());
    }

    let origin = state.players[actor_index]
        .position
        .ok_or(CoreError::InvalidState("living actor has no position"))?;
    let origin_terrain = state.terrain[map_index(origin, state.map_size)?];
    let range = if origin_terrain == Terrain::HighGround {
        BASE_SHOT_RANGE.saturating_add(1)
    } else {
        BASE_SHOT_RANGE
    };
    let mut cursor = origin;
    let mut hit_something = false;
    let mut hit_player = false;

    for _ in 0..range {
        let Some(next) = step_position(cursor, direction, state.map_size) else {
            break;
        };
        cursor = next;
        let terrain_index = map_index(cursor, state.map_size)?;
        let current_terrain = state.terrain[terrain_index];
        let props = current_terrain.properties();

        // 1. Terrain interaction, hardness check and legality protection
        if props.layer == TerrainLayer::AboveGround && props.hardness.is_some() {
            let hardness = props.hardness.unwrap_or(0);
            let Some(target_terrain) = props.transform_on_destroy else {
                hit_something = true;
                break;
            };

            if piercing {
                if hardness <= 2 {
                    // Armor-piercing bullet: destroys hardness <= 2 (including Wall and Crate)
                    state.terrain[terrain_index] = target_terrain;
                    push_event(
                        state,
                        GameEvent::TerrainChanged {
                            position: cursor,
                            from: current_terrain,
                            to: target_terrain,
                        },
                    )?;
                    hit_something = true;
                    let impact_trigger = events::TriggerPoint::ProjectileImpact {
                        shooter_id: actor_id,
                        position: cursor,
                        hit_terrain: current_terrain,
                        hit_player: None,
                    };
                    events::EventTreePipeline::default().run(state, impact_trigger)?;
                    // AP bullet penetrates through! Does not break.
                } else {
                    hit_something = true;
                    break;
                }
            } else if hardness <= 1 {
                // Normal bullet destroys fragile terrain (e.g. Crate) and stops
                state.terrain[terrain_index] = target_terrain;
                push_event(
                    state,
                    GameEvent::TerrainChanged {
                        position: cursor,
                        from: current_terrain,
                        to: target_terrain,
                    },
                )?;
                hit_something = true;
                let impact_trigger = events::TriggerPoint::ProjectileImpact {
                    shooter_id: actor_id,
                    position: cursor,
                    hit_terrain: current_terrain,
                    hit_player: None,
                };
                events::EventTreePipeline::default().run(state, impact_trigger)?;
                break;
            } else {
                // Normal bullet cannot destroy hardness > 1 (e.g. Wall) and stops
                hit_something = true;
                break;
            }
        }

        if state.terrain[terrain_index] == Terrain::Water
            || state.terrain[terrain_index] == Terrain::Ice
        {
            continue;
        }
        if let Some(target_index) = living_player_index_at(state, cursor, Some(actor_id)) {
            eliminate_player(
                state,
                target_index,
                EliminationCause::Shot,
                Some(actor_id),
                true,
            )?;
            hit_something = true;
            hit_player = true;
            break;
        }
    }

    if ricochet && !hit_player {
        let mut closest: Option<(usize, u32)> = None;
        for (idx, p) in state.players.iter().enumerate() {
            if idx == actor_index || p.status != PlayerStatus::Alive {
                continue;
            }
            let Some(pos) = p.position else {
                continue;
            };
            let dist = u32::from(pos.x.abs_diff(cursor.x) + pos.y.abs_diff(cursor.y));
            if closest.as_ref().is_none_or(|(_, best)| dist < *best) {
                closest = Some((idx, dist));
            }
        }
        if let Some((target_index, _)) = closest {
            eliminate_player(
                state,
                target_index,
                EliminationCause::Shot,
                Some(actor_id),
                true,
            )?;
            hit_something = true;
        }
    }

    if !hit_something {
        push_event(state, GameEvent::ShotMissed { actor_id })?;
    }
    Ok(())
}

fn apply_recoil(
    state: &mut GameState,
    actor_index: usize,
    direction: Direction,
) -> Result<(), CoreError> {
    if state.players[actor_index].status != PlayerStatus::Alive {
        return Ok(());
    }
    let actor_id = state.players[actor_index].id;
    let from = state.players[actor_index]
        .position
        .ok_or(CoreError::InvalidState("living actor has no position"))?;
    let Some(target) = step_position(from, direction, state.map_size) else {
        return Ok(());
    };
    let target_index = map_index(target, state.map_size)?;
    let terrain = state.terrain[target_index];
    if terrain == Terrain::Wall || terrain == Terrain::Crate {
        return Ok(());
    }
    if living_player_index_at(state, target, Some(actor_id)).is_some() {
        return Ok(());
    }
    complete_move_step(state, actor_index, from, target, direction)
}

fn resolve_water(state: &mut GameState, actor_index: usize) -> Result<(), CoreError> {
    if state.players[actor_index].status != PlayerStatus::Alive {
        return Ok(());
    }
    let position = state.players[actor_index]
        .position
        .ok_or(CoreError::InvalidState("living actor has no position"))?;
    let terrain = state.terrain[map_index(position, state.map_size)?];
    if terrain == Terrain::Water {
        state.players[actor_index].water_turns =
            state.players[actor_index].water_turns.saturating_add(1);
        if state.players[actor_index].water_turns >= 2 {
            let eliminated =
                eliminate_player(state, actor_index, EliminationCause::Drowned, None, true)?;
            if !eliminated {
                state.players[actor_index].water_turns = 0;
            }
        }
    } else {
        state.players[actor_index].water_turns = 0;
    }
    Ok(())
}

fn eliminate_player(
    state: &mut GameState,
    player_index: usize,
    cause: EliminationCause,
    by_player_id: Option<PlayerId>,
    allow_shield: bool,
) -> Result<bool, CoreError> {
    let player_id = state.players[player_index].id;
    if allow_shield && state.players[player_index].has_shield {
        state.players[player_index].has_shield = false;
        push_event(state, GameEvent::ShieldConsumed { player_id, cause })?;
        return Ok(false);
    }
    state.players[player_index].status = PlayerStatus::Eliminated;
    state.players[player_index].position = None;
    state.rounds_without_elimination = 0;
    push_event(
        state,
        GameEvent::PlayerEliminated {
            player_id,
            cause,
            by_player_id,
        },
    )?;
    Ok(true)
}

fn finish_or_advance_turn(state: &mut GameState) -> Result<(), CoreError> {
    let living_players = state
        .players
        .iter()
        .filter(|player| player.status == PlayerStatus::Alive)
        .map(|player| player.id)
        .collect::<Vec<_>>();
    if living_players.len() <= 1 {
        let winner_id = living_players.first().copied();
        state.status = GameStatus::Finished { winner_id };
        append_record(
            state,
            GameRecordContent::Notification {
                level: NotificationLevel::Success,
                notification: GameNotification::MatchFinished { winner_id },
            },
        )?;
        return Ok(());
    }

    let turn_count = state.turn_order.len();
    for step in 1..=turn_count {
        let raw_index = state.current_turn_index + step;
        let candidate_index = raw_index % turn_count;
        let candidate_id = state.turn_order[candidate_index];
        let candidate_player_index = player_index(state, candidate_id)?;
        if state.players[candidate_player_index].status == PlayerStatus::Alive {
            if raw_index >= turn_count {
                state.round = state
                    .round
                    .checked_add(1)
                    .ok_or(CoreError::InvalidState("round overflow"))?;
                state.rounds_without_elimination =
                    state.rounds_without_elimination.saturating_add(1);
                let round_trigger = events::TriggerPoint::RoundStart { round: state.round };
                events::EventTreePipeline::default().run(state, round_trigger)?;
            }
            state.current_turn_index = candidate_index;
            append_record(
                state,
                GameRecordContent::Turn {
                    player_id: candidate_id,
                    round: state.round,
                },
            )?;
            return Ok(());
        }
    }
    Err(CoreError::InvalidState(
        "no living player found for next turn",
    ))
}

pub(crate) fn push_event(state: &mut GameState, event: GameEvent) -> Result<u64, CoreError> {
    append_record(state, GameRecordContent::Event { event })
}

pub(crate) fn append_record(
    state: &mut GameState,
    content: GameRecordContent,
) -> Result<u64, CoreError> {
    let sequence = state.next_record_sequence;
    state.next_record_sequence = state
        .next_record_sequence
        .checked_add(1)
        .ok_or(CoreError::InvalidState("record sequence overflow"))?;
    state.records.push(GameRecord { sequence, content });
    Ok(sequence)
}

pub(crate) fn player_index(state: &GameState, player_id: PlayerId) -> Result<usize, CoreError> {
    state
        .players
        .iter()
        .position(|player| player.id == player_id)
        .ok_or(CoreError::UnknownPlayer(player_id))
}

pub(crate) fn living_player_index_at(
    state: &GameState,
    position: Position,
    excluded: Option<PlayerId>,
) -> Option<usize> {
    state.players.iter().position(|player| {
        player.status == PlayerStatus::Alive
            && player.position == Some(position)
            && Some(player.id) != excluded
    })
}

pub(crate) fn map_index(position: Position, map_size: u8) -> Result<usize, CoreError> {
    if position.x >= map_size || position.y >= map_size {
        return Err(CoreError::InvalidState("position lies outside the map"));
    }
    Ok(usize::from(position.y) * usize::from(map_size) + usize::from(position.x))
}

pub(crate) fn position_from_index(index: usize, map_size: u8) -> Result<Position, CoreError> {
    let size = usize::from(map_size);
    let x = u8::try_from(index % size)
        .map_err(|_| CoreError::InvalidState("x coordinate does not fit in u8"))?;
    let y = u8::try_from(index / size)
        .map_err(|_| CoreError::InvalidState("y coordinate does not fit in u8"))?;
    Ok(Position { x, y })
}

pub(crate) fn step_position(
    position: Position,
    direction: Direction,
    map_size: u8,
) -> Option<Position> {
    let (delta_x, delta_y) = match direction {
        Direction::Up => (0_i16, -1_i16),
        Direction::Down => (0, 1),
        Direction::Left => (-1, 0),
        Direction::Right => (1, 0),
    };
    let next_x = i16::from(position.x) + delta_x;
    let next_y = i16::from(position.y) + delta_y;
    if next_x < 0 || next_y < 0 || next_x >= i16::from(map_size) || next_y >= i16::from(map_size) {
        return None;
    }
    Some(Position {
        x: u8::try_from(next_x).ok()?,
        y: u8::try_from(next_y).ok()?,
    })
}

fn place_terrain(
    terrain: &mut [Terrain],
    value: Terrain,
    rng_state: &mut u64,
) -> Result<(), CoreError> {
    let available = terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain)| (*terrain == Terrain::Empty).then_some(index))
        .collect::<Vec<_>>();
    let selected = available
        .get(random_index(rng_state, available.len())?)
        .copied()
        .ok_or(CoreError::InvalidState("no empty terrain cell available"))?;
    terrain[selected] = value;
    Ok(())
}

fn choose_empty_cell(
    terrain: &[Terrain],
    players: &[PlayerState],
    map_size: u8,
    rng_state: &mut u64,
) -> Result<usize, CoreError> {
    let available = terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain)| {
            if *terrain != Terrain::Empty {
                return None;
            }
            let position = position_from_index(index, map_size).ok()?;
            (!players
                .iter()
                .any(|player| player.position == Some(position)))
            .then_some(index)
        })
        .collect::<Vec<_>>();
    available
        .get(random_index(rng_state, available.len())?)
        .copied()
        .ok_or(CoreError::InvalidState("no player spawn cell available"))
}

fn derive_stream(seed: u64, label: u64) -> u64 {
    let mut state = seed ^ label;
    next_random(&mut state)
}

pub(crate) fn random_index(state: &mut u64, upper_bound: usize) -> Result<usize, CoreError> {
    if upper_bound == 0 {
        return Err(CoreError::InvalidState("random choice has no candidates"));
    }
    let upper = u64::try_from(upper_bound)
        .map_err(|_| CoreError::InvalidState("random bound does not fit in u64"))?;
    usize::try_from(next_random(state) % upper)
        .map_err(|_| CoreError::InvalidState("random index does not fit in usize"))
}

pub(crate) fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roulette_domain::{PlayerKind, PlayerSetup};

    fn test_config(seed: u64) -> CreateGameConfig {
        CreateGameConfig {
            seed,
            players: vec![
                PlayerSetup {
                    name: "Human".to_owned(),
                    kind: PlayerKind::Human,
                },
                PlayerSetup {
                    name: "Bot 1".to_owned(),
                    kind: PlayerKind::Bot,
                },
                PlayerSetup {
                    name: "Bot 2".to_owned(),
                    kind: PlayerKind::Bot,
                },
            ],
        }
    }

    #[test]
    fn creation_is_deterministic() {
        let first = GameEngine::create_game(test_config(42)).expect("first game");
        let second = GameEngine::create_game(test_config(42)).expect("second game");
        assert_eq!(first, second);
    }

    #[test]
    fn map_contains_unique_player_positions() {
        let state = GameEngine::create_game(test_config(7)).expect("game");
        assert_eq!(
            state.terrain.len(),
            usize::from(state.map_size) * usize::from(state.map_size)
        );
        let mut positions = state
            .players
            .iter()
            .filter_map(|player| player.position)
            .collect::<Vec<_>>();
        positions.sort_by_key(|position| (position.y, position.x));
        positions.dedup();
        assert_eq!(positions.len(), state.players.len());
    }

    #[test]
    fn third_consecutive_shot_is_forced_empty() {
        let mut state = GameEngine::create_game(test_config(11)).expect("game");
        state.players[0].consecutive_shots = 2;
        let records = GameEngine::apply_command(
            &mut state,
            PlayerId(1),
            PlayerCommand::Shoot {
                direction: Direction::Up,
            },
        )
        .expect("shot");
        assert!(records.iter().any(|record| matches!(
            record,
            GameRecord {
                content: GameRecordContent::Event {
                    event: GameEvent::EmptyChamber {
                        actor_id: PlayerId(1),
                        forced: true
                    }
                },
                ..
            }
        )));
    }

    #[test]
    fn blocked_move_consumes_turn_and_revision() {
        let mut state = GameEngine::create_game(test_config(15)).expect("game");
        state.players[0].position = Some(Position { x: 0, y: 0 });
        let records = GameEngine::apply_command(
            &mut state,
            PlayerId(1),
            PlayerCommand::Move {
                direction: Direction::Up,
            },
        )
        .expect("blocked move");
        assert_eq!(state.revision, 1);
        assert!(records.iter().any(|record| matches!(
            record,
            GameRecord {
                content: GameRecordContent::Event {
                    event: GameEvent::MoveBlocked {
                        actor_id: PlayerId(1),
                        reason: BlockReason::Boundary
                    }
                },
                ..
            }
        )));
        assert!(matches!(
            records.first(),
            Some(GameRecord {
                content: GameRecordContent::Action {
                    actor_id: PlayerId(1),
                    command: PlayerCommand::Move { .. }
                },
                ..
            })
        ));
    }

    #[test]
    fn action_records_do_not_require_an_event_record() {
        let mut state = GameEngine::create_game(test_config(21)).expect("game");
        assert!(matches!(
            state.records.first(),
            Some(GameRecord {
                sequence: 1,
                content: GameRecordContent::Notification { .. }
            })
        ));

        let records =
            GameEngine::apply_command(&mut state, PlayerId(1), PlayerCommand::Wait).expect("wait");
        assert_eq!(records.len(), 2);
        assert!(matches!(
            records[0],
            GameRecord {
                content: GameRecordContent::Action {
                    command: PlayerCommand::Wait,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            records[1],
            GameRecord {
                content: GameRecordContent::Turn { .. },
                ..
            }
        ));
        assert!(
            state
                .records
                .windows(2)
                .all(|pair| pair[1].sequence == pair[0].sequence + 1)
        );
    }

    #[test]
    fn identical_commands_keep_states_equal() {
        let mut first = GameEngine::create_game(test_config(99)).expect("first game");
        let mut second = first.clone();
        for _ in 0..9 {
            let first_actor = GameEngine::current_player_id(&first).expect("first actor");
            let second_actor = GameEngine::current_player_id(&second).expect("second actor");
            GameEngine::apply_command(&mut first, first_actor, PlayerCommand::Wait)
                .expect("first wait");
            GameEngine::apply_command(&mut second, second_actor, PlayerCommand::Wait)
                .expect("second wait");
        }
        assert_eq!(first, second);
    }

    #[test]
    fn event_pool_dampening_shifts_probabilities_toward_closure() {
        use crate::events::{EventPool, PoolEntry};

        let pool = EventPool::new(
            "test_dampening",
            vec![
                PoolEntry::normal("evt_blast", 40),
                PoolEntry::dampener("evt_settles", 10, 30),
            ],
        );

        let mut rng_depth0 = 0x1234_5678;
        let mut depth0_settles = 0;
        let iterations = 1000;
        for _ in 0..iterations {
            if pool.roll(0, 0, &mut rng_depth0).expect("roll").0 == "evt_settles" {
                depth0_settles += 1;
            }
        }

        let mut rng_depth4 = 0x1234_5678;
        let mut depth4_settles = 0;
        for _ in 0..iterations {
            if pool.roll(4, 0, &mut rng_depth4).expect("roll").0 == "evt_settles" {
                depth4_settles += 1;
            }
        }

        // At depth 0: eff_weight(blast) = 40, eff_weight(settles) = 10 -> ~20%
        // At depth 4: eff_weight(blast) = 40 / 5 = 8, eff_weight(settles) = 10 + 120 = 130 -> 130/138 ~ 94%
        assert!(depth0_settles < 300, "expected ~200, got {depth0_settles}");
        assert!(depth4_settles > 850, "expected >850, got {depth4_settles}");
    }

    #[test]
    fn test_low_depth_suppression_of_dampeners() {
        use crate::events::{TriggerPoint, resolve_pool};

        let state = GameEngine::create_game(test_config(77)).expect("game");
        let pos = state.players[0].position.unwrap();
        let free_dir = [
            Direction::Down,
            Direction::Right,
            Direction::Left,
            Direction::Up,
        ]
        .into_iter()
        .find(|dir| {
            if let Some(target) = crate::step_position(pos, *dir, state.map_size) {
                let t_idx = crate::map_index(target, state.map_size).unwrap();
                state.terrain[t_idx] != Terrain::Wall
            } else {
                false
            }
        })
        .expect("free dir");

        let move_trigger = TriggerPoint::ActionIntent {
            actor_id: state.players[0].id,
            command: PlayerCommand::Move {
                direction: free_dir,
            },
        };
        let pool = resolve_pool(&move_trigger, &state).expect("pool");
        assert_eq!(pool.name, "move_intent");
        let mut rng = 42;
        let (_, total_weight, _, candidates) = pool.roll_with_trace(0, 0, &mut rng);

        // At depth 0, stalemate 0: dampener (nothing_happens) and stumble should be low
        let nothing_cand = candidates
            .iter()
            .find(|c| c.event_id.0 == "evt_nothing_happens")
            .expect("cand");
        let stumble_cand = candidates
            .iter()
            .find(|c| c.event_id.0 == "evt_stumble_trip")
            .expect("cand");

        // nothing_happens (6/60 = 10%), stumble (4/60 = 6.6%)
        assert!(
            nothing_cand.probability_permille <= 120,
            "nothing happens should be <= 12%, got {}",
            nothing_cand.probability_permille
        );
        assert!(
            stumble_cand.probability_permille <= 80,
            "stumble should be <= 8%, got {}",
            stumble_cand.probability_permille
        );
        assert!(total_weight > 0);
    }

    #[test]
    fn test_stalemate_escalation_boosts_lethal_weights() {
        use crate::events::{TriggerPoint, resolve_pool};

        let state = GameEngine::create_game(test_config(88)).expect("game");
        let round_trigger = TriggerPoint::RoundStart { round: 2 };
        let pool = resolve_pool(&round_trigger, &state).expect("pool");

        let mut rng = 123;
        let (_, _, _, cands_peace0) = pool.roll_with_trace(0, 0, &mut rng);
        let (_, _, _, cands_peace4) = pool.roll_with_trace(0, 4, &mut rng);

        let meteor_peace0 = cands_peace0
            .iter()
            .find(|c| c.event_id.0 == "evt_meteor_strike")
            .expect("meteor");
        let meteor_peace4 = cands_peace4
            .iter()
            .find(|c| c.event_id.0 == "evt_meteor_strike")
            .expect("meteor");

        assert!(meteor_peace0.is_lethal);
        assert!(meteor_peace4.is_lethal);

        // Base 15 -> with stalemate 4: 15 + 4 * 25 = 115!
        assert_eq!(meteor_peace0.effective_weight, 15);
        assert_eq!(meteor_peace4.effective_weight, 115);
        assert!(
            meteor_peace4.probability_permille > meteor_peace0.probability_permille * 2,
            "meteor probability should more than double in stalemate"
        );
    }

    #[test]
    fn test_elimination_resets_stalemate_counter() {
        let mut state = GameEngine::create_game(test_config(66)).expect("game");
        assert_eq!(state.rounds_without_elimination, 0);

        // Advance turn past all living players to increment round
        let turn_count = state.turn_order.len();
        for _ in 0..turn_count {
            let actor_id = state.players[state.current_turn_index].id;
            GameEngine::apply_command(&mut state, actor_id, PlayerCommand::Wait).expect("wait");
        }

        assert_eq!(state.round, 2);
        assert_eq!(state.rounds_without_elimination, 1);

        // Eliminate player 1
        let killer_id = state.players[1].id;
        eliminate_player(
            &mut state,
            0,
            EliminationCause::Shot,
            Some(killer_id),
            false,
        )
        .expect("elim");

        // Counter should immediately reset to 0
        assert_eq!(state.rounds_without_elimination, 0);
    }

    #[test]
    fn test_wall_crash_pool_and_lethal_majority() {
        use crate::events::{TriggerPoint, resolve_pool};

        let state = GameEngine::create_game(test_config(77)).expect("game");
        let pos = state.players[0].position.unwrap();
        // Pick an obstructed direction (boundary or wall)
        let blocked_dir = [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ]
        .into_iter()
        .find(|dir| {
            let target = crate::step_position(pos, *dir, state.map_size);
            target.is_none_or(|t| {
                let t_idx = crate::map_index(t, state.map_size).unwrap();
                state.terrain[t_idx] == Terrain::Wall
            })
        })
        .expect("blocked dir");

        let move_trigger = TriggerPoint::ActionIntent {
            actor_id: state.players[0].id,
            command: PlayerCommand::Move {
                direction: blocked_dir,
            },
        };
        let pool = resolve_pool(&move_trigger, &state).expect("pool");
        assert_eq!(pool.name, "wall_crash");

        let mut rng = 42;
        let (_, total_weight, _, candidates) = pool.roll_with_trace(0, 0, &mut rng);
        assert!(total_weight > 0);

        // Sum lethal probabilities vs survival probabilities
        let lethal_permille: u32 = candidates
            .iter()
            .filter(|c| c.is_lethal)
            .map(|c| c.probability_permille)
            .sum();
        let survive_permille: u32 = candidates
            .iter()
            .filter(|c| !c.is_lethal)
            .map(|c| c.probability_permille)
            .sum();

        // Lethal events should dominate (majority: >= 65%), with small survival probability (> 0% and <= 35%)
        assert!(
            lethal_permille >= 650,
            "lethal events should dominate wall crash: {lethal_permille}"
        );
        assert!(
            survive_permille > 0 && survive_permille <= 350,
            "survival should be minor: {survive_permille}"
        );
    }

    #[test]
    fn test_sudden_landmine_eliminates_actor() {
        use crate::events::TriggerPoint;

        let mut state = GameEngine::create_game(test_config(88)).expect("game");
        let actor_id = state.players[0].id;
        let trigger = TriggerPoint::ActionIntent {
            actor_id,
            command: PlayerCommand::Move {
                direction: Direction::Down,
            },
        };

        // Run sudden landmine effect directly
        let mut outcome = crate::events::PipelineOutcome::default();
        let desc =
            crate::events::pipeline::apply_sudden_landmine(&mut state, &trigger, &mut outcome)
                .expect("mine");

        assert!(
            outcome.action_canceled,
            "action should be canceled by mine explosion"
        );
        assert_eq!(state.players[0].status, PlayerStatus::Eliminated);
        assert!(desc.contains("暗雷轰然引爆"));
    }

    #[test]
    fn test_empty_terrain_has_high_nothing_happens_rate() {
        use crate::events::{TriggerPoint, resolve_pool};

        let state = GameEngine::create_game(test_config(55)).expect("game");
        let enter_trigger = TriggerPoint::TerrainEntered {
            actor_id: state.players[0].id,
            from: Position { x: 1, y: 1 },
            to: Position { x: 1, y: 2 },
            terrain: Terrain::Empty,
            direction: Direction::Down,
        };
        let pool = resolve_pool(&enter_trigger, &state).expect("pool");
        assert_eq!(pool.name, "terrain_enter_empty");
        let mut rng = 123;
        let (_, _, _, candidates) = pool.roll_with_trace(0, 0, &mut rng);
        let nothing_cand = candidates
            .iter()
            .find(|c| c.event_id.0 == "evt_nothing_happens")
            .expect("nothing_happens");
        assert_eq!(nothing_cand.base_weight, 99);

        let exit_trigger = TriggerPoint::TerrainExited {
            actor_id: state.players[0].id,
            from: Position { x: 1, y: 1 },
            to: Position { x: 1, y: 2 },
            terrain: Terrain::Empty,
            direction: Direction::Down,
        };
        let exit_pool = resolve_pool(&exit_trigger, &state).expect("exit pool");
        assert_eq!(exit_pool.name, "terrain_exit_empty");
        let (_, _, _, exit_cands) = exit_pool.roll_with_trace(0, 0, &mut rng);
        let exit_nothing = exit_cands
            .iter()
            .find(|c| c.event_id.0 == "evt_nothing_happens")
            .expect("nothing_happens");
        assert_eq!(exit_nothing.probability_permille, 1000);
    }

    #[test]
    fn test_mine_terrain_enter_triggers_detonation_or_dud() {
        use crate::events::{TriggerPoint, resolve_pool};

        let mut state = GameEngine::create_game(test_config(99)).expect("game");
        let mine_pos = Position { x: 2, y: 2 };
        let mine_idx = map_index(mine_pos, state.map_size).unwrap();
        state.terrain[mine_idx] = Terrain::Mine;

        let enter_trigger = TriggerPoint::TerrainEntered {
            actor_id: state.players[0].id,
            from: Position { x: 2, y: 1 },
            to: mine_pos,
            terrain: Terrain::Mine,
            direction: Direction::Down,
        };
        let pool = resolve_pool(&enter_trigger, &state).expect("pool");
        assert_eq!(pool.name, "terrain_enter_mine");

        let mut rng = 42;
        let (_, _, _, candidates) = pool.roll_with_trace(0, 0, &mut rng);
        let det_cand = candidates
            .iter()
            .find(|c| c.event_id.0 == "evt_mine_detonation")
            .expect("det");
        assert!(det_cand.is_lethal);
        assert!(det_cand.probability_permille >= 800);
    }

    #[test]
    fn test_terrain_exited_and_entered_lifecycle() {
        let mut state = GameEngine::create_game(test_config(101)).expect("game");
        let p0_pos = state.players[0].position.unwrap();
        let (dir, target) = [
            Direction::Down,
            Direction::Up,
            Direction::Right,
            Direction::Left,
        ]
        .into_iter()
        .find_map(|d| {
            let t = crate::step_position(p0_pos, d, state.map_size)?;
            Some((d, t))
        })
        .expect("valid target");
        let target_idx = map_index(target, state.map_size).unwrap();
        state.terrain[target_idx] = Terrain::Empty;

        // Perform move step
        complete_move_step(&mut state, 0, p0_pos, target, dir).expect("move step");
        assert_eq!(state.players[0].position, Some(target));

        // Traces should record TerrainExited and TerrainEntered
        let trace_descs: Vec<&str> = state
            .event_traces
            .iter()
            .map(|t| t.root_trigger_desc.as_str())
            .collect();
        assert!(
            trace_descs.iter().any(|d| d.contains("离开")),
            "should have a TerrainExited trace: {trace_descs:?}"
        );
        assert!(
            trace_descs.iter().any(|d| d.contains("踏入")),
            "should have a TerrainEntered trace: {trace_descs:?}"
        );
    }

    #[test]
    fn event_pipeline_blizzard_freezes_water_to_ice() {
        use crate::events::{EventTreePipeline, TriggerPoint};

        let mut state = GameEngine::create_game(test_config(33)).expect("game");
        let water_pos = Position { x: 1, y: 1 };
        let water_idx = map_index(water_pos, state.map_size).expect("idx");
        state.terrain[water_idx] = Terrain::Water;

        // Force blizzard by directly calling the blizzard secondary trigger pipeline
        let trigger = TriggerPoint::SecondaryTrigger {
            tag: "freeze_waters".to_owned(),
            actor_id: None,
            position: None,
        };
        let outcome = EventTreePipeline::default()
            .run(&mut state, trigger)
            .expect("pipeline");

        assert_eq!(outcome.events_resolved, 1);
        assert_eq!(state.terrain[water_idx], Terrain::Ice);
        assert!(state.records.iter().any(|r| matches!(
            r,
            GameRecord {
                content: GameRecordContent::Event {
                    event: GameEvent::TerrainChanged {
                        from: Terrain::Water,
                        to: Terrain::Ice,
                        ..
                    }
                },
                ..
            }
        )));
    }

    #[test]
    fn event_pipeline_ice_slide_moves_actor_forward() {
        use crate::events::{EventTreePipeline, TriggerPoint};

        let mut state = GameEngine::create_game(test_config(44)).expect("game");
        let p1 = state.players[0].id;
        state.players[0].position = Some(Position { x: 0, y: 1 });

        let ice_pos = Position { x: 0, y: 1 };
        let ice_idx = map_index(ice_pos, state.map_size).expect("idx");
        state.terrain[ice_idx] = Terrain::Ice;

        let empty_pos = Position { x: 0, y: 2 };
        let empty_idx = map_index(empty_pos, state.map_size).expect("idx");
        state.terrain[empty_idx] = Terrain::Empty;

        let ice_trigger = TriggerPoint::TerrainEntered {
            actor_id: p1,
            from: Position { x: 0, y: 0 },
            to: ice_pos,
            terrain: Terrain::Ice,
            direction: Direction::Down,
        };

        let _outcome = EventTreePipeline::default()
            .run(&mut state, ice_trigger)
            .expect("ice pipeline");

        let final_pos = state.players[0].position.expect("pos");
        assert!(
            final_pos == Position { x: 0, y: 1 } || final_pos == Position { x: 0, y: 2 },
            "player landed at unexpected position {final_pos:?}",
        );
        assert!(!state.event_traces.is_empty());
        let last_trace = state.event_traces.last().unwrap();
        assert_eq!(last_trace.nodes[0].pool_name, "terrain_enter_ice");
        assert!(!last_trace.nodes[0].candidates.is_empty());
    }

    #[test]
    fn event_pipeline_records_diagnostic_tree_traces() {
        use crate::events::{EventTreePipeline, TriggerPoint};

        let mut state = GameEngine::create_game(test_config(99)).expect("game");
        let trigger = TriggerPoint::RoundStart { round: 2 };

        let _outcome = EventTreePipeline::default()
            .run(&mut state, trigger)
            .expect("round start pipeline");

        assert_eq!(state.event_traces.len(), 1);
        let trace = &state.event_traces[0];
        assert_eq!(trace.round, 1);
        assert!(!trace.nodes.is_empty());
        let root_node = &trace.nodes[0];
        assert_eq!(root_node.wave, 0);
        assert!(root_node.total_weight > 0);
        for cand in &root_node.candidates {
            assert!(cand.probability_permille <= 1000);
        }
    }

    #[test]
    fn event_pipeline_heatwave_melts_ice_to_water() {
        use crate::events::{EventTreePipeline, TriggerPoint};

        let mut state = GameEngine::create_game(test_config(55)).expect("game");
        let ice_pos = Position { x: 2, y: 2 };
        let ice_idx = map_index(ice_pos, state.map_size).expect("idx");
        state.terrain[ice_idx] = Terrain::Ice;

        let trigger = TriggerPoint::SecondaryTrigger {
            tag: "melt_ices".to_owned(),
            actor_id: None,
            position: None,
        };

        let outcome = EventTreePipeline::default()
            .run(&mut state, trigger)
            .expect("heatwave pipeline");

        assert_eq!(outcome.events_resolved, 1);
        assert_eq!(state.weather, Weather::Heatwave);
        assert_eq!(state.terrain[ice_idx], Terrain::Water);
    }

    #[test]
    fn test_normal_bullet_cannot_destroy_wall() {
        let mut state = GameEngine::create_game(test_config(42)).expect("game");
        let p0_pos = Position { x: 1, y: 1 };
        let wall_pos = Position { x: 1, y: 2 };
        let target_pos = Position { x: 1, y: 3 };

        state.players[0].position = Some(p0_pos);
        state.players[1].position = Some(target_pos);

        let wall_idx = map_index(wall_pos, state.map_size).unwrap();
        state.terrain[wall_idx] = Terrain::Wall;

        state.players[0].consecutive_shots = 0;
        state.rng.combat = 1;

        apply_shot(&mut state, 0, Direction::Down, false, false).expect("shot");

        assert_eq!(state.terrain[wall_idx], Terrain::Wall);
        assert_eq!(state.players[1].status, PlayerStatus::Alive);
    }

    #[test]
    fn test_piercing_bullet_destroys_wall_and_hits_target_behind() {
        let mut state = GameEngine::create_game(test_config(42)).expect("game");
        let p0_pos = Position { x: 1, y: 1 };
        let wall_pos = Position { x: 1, y: 2 };
        let target_pos = Position { x: 1, y: 3 };

        state.players[0].position = Some(p0_pos);
        state.players[1].position = Some(target_pos);

        let wall_idx = map_index(wall_pos, state.map_size).unwrap();
        state.terrain[wall_idx] = Terrain::Wall;

        state.players[0].consecutive_shots = 0;
        state.rng.combat = 1;

        apply_shot(&mut state, 0, Direction::Down, true, false).expect("shot");

        assert_eq!(state.terrain[wall_idx], Terrain::Empty);
        assert_eq!(state.players[1].status, PlayerStatus::Eliminated);
    }

    #[test]
    fn test_bullet_does_not_destroy_underground_mine() {
        let mut state = GameEngine::create_game(test_config(42)).expect("game");
        let p0_pos = Position { x: 1, y: 1 };
        let mine_pos = Position { x: 1, y: 2 };

        state.players[0].position = Some(p0_pos);
        let mine_idx = map_index(mine_pos, state.map_size).unwrap();
        state.terrain[mine_idx] = Terrain::Mine;

        state.players[0].consecutive_shots = 0;
        state.rng.combat = 1;

        apply_shot(&mut state, 0, Direction::Down, true, false).expect("shot");

        assert_eq!(state.terrain[mine_idx], Terrain::Mine);
    }

    #[test]
    fn test_normal_bullet_destroys_crate_and_stops() {
        let mut state = GameEngine::create_game(test_config(42)).expect("game");
        let p0_pos = Position { x: 1, y: 1 };
        let crate_pos = Position { x: 1, y: 2 };
        let target_pos = Position { x: 1, y: 3 };

        state.players[0].position = Some(p0_pos);
        state.players[1].position = Some(target_pos);

        let crate_idx = map_index(crate_pos, state.map_size).unwrap();
        state.terrain[crate_idx] = Terrain::Crate;

        state.players[0].consecutive_shots = 0;
        state.rng.combat = 1;

        apply_shot(&mut state, 0, Direction::Down, false, false).expect("shot");

        // Crate is destroyed (no longer Crate, could be Empty or reveal a surprise mine/medkit)
        assert_ne!(state.terrain[crate_idx], Terrain::Crate);
        assert_eq!(state.players[1].status, PlayerStatus::Alive);
    }
}
