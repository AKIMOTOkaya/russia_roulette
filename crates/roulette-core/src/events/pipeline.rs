//! Breadth-First-Search event tree pipeline.

use std::collections::HashMap;

use roulette_domain::{
    Direction, GameEvent, GameRecordContent, GameState, PlayerCommand, Position, Terrain, Weather,
};

use crate::CoreError;
use crate::events::catalog::EventRegistry;
use crate::events::pools::{TriggerPoint, resolve_pool};

/// Node in the Breadth-First-Search event resolution tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventNode {
    /// Unique sequence identifier of this tree node within the pipeline run.
    pub node_id: u32,
    /// Parent node ID if this node was triggered as a secondary child reaction.
    pub parent_id: Option<u32>,
    /// Chronological BFS wave (0: root trigger, 1: primary reactions, 2+: secondary waves).
    pub wave: u32,
    /// Path depth accumulated along this branch from root.
    pub path_depth: u32,
    /// Sequence number of the parent event record that prompted this reaction.
    pub parent_sequence: Option<u64>,
    /// Contextual trigger point for this node.
    pub trigger: TriggerPoint,
}

/// Resulting modifications produced by the event pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PipelineOutcome {
    /// Command override if an `ActionIntent` event altered direction or behavior.
    pub override_command: Option<PlayerCommand>,
    /// Whether the action was canceled (e.g. misfire).
    pub action_canceled: bool,
    /// Total dramatic events emitted into the record log.
    pub events_resolved: usize,
}

/// Breadth-First-Search execution pipeline for resolving dramatic event trees.
pub struct EventTreePipeline {
    /// Maximum cumulative path depth allowed along any single branch.
    pub max_branch_depth: u32,
    /// Hard safety cap on total nodes processed in a single run.
    pub max_total_nodes: usize,
}

impl Default for EventTreePipeline {
    fn default() -> Self {
        Self {
            max_branch_depth: 6,
            max_total_nodes: 32,
        }
    }
}

impl EventTreePipeline {
    /// Executes the event tree pipeline starting from a root trigger point.
    ///
    /// # Errors
    ///
    /// Returns an error if state modification or record appending violates Core invariants.
    pub fn run(
        &self,
        state: &mut GameState,
        root_trigger: TriggerPoint,
    ) -> Result<PipelineOutcome, CoreError> {
        let mut outcome = PipelineOutcome::default();
        let mut wave_queue = vec![EventNode {
            node_id: 1,
            parent_id: None,
            wave: 0,
            path_depth: 0,
            parent_sequence: None,
            trigger: root_trigger,
        }];

        let mut next_node_id = 2;
        let mut current_wave = 0;
        let mut total_processed = 0;
        let mut last_emitted_sequences: HashMap<u32, u64> = HashMap::new();

        while !wave_queue.is_empty() && total_processed < self.max_total_nodes {
            let mut next_wave = Vec::new();

            for node in wave_queue {
                total_processed += 1;
                if total_processed > self.max_total_nodes {
                    break;
                }

                let Some(pool) = resolve_pool(&node.trigger, state) else {
                    continue;
                };

                let Some(event_id) = pool.roll(node.path_depth, &mut state.rng.events) else {
                    continue;
                };

                // Silent no-op dampener: closes the branch without creating a record
                if event_id.0 == "evt_nothing_happens" {
                    continue;
                }

                let Some(def) = EventRegistry::get(&event_id) else {
                    continue;
                };

                let parent_seq = node
                    .parent_id
                    .and_then(|p_id| last_emitted_sequences.get(&p_id).copied())
                    .or(node.parent_sequence);

                let event_record_seq = crate::append_record(
                    state,
                    GameRecordContent::Event {
                        event: GameEvent::DramaticEvent {
                            event_id: def.id.clone(),
                            tier: def.tier,
                            title: def.title.to_owned(),
                            narrative: def.narrative.to_owned(),
                            actor_id: node.trigger.actor_id(),
                            position: node.trigger.position(),
                            wave: current_wave,
                            parent_sequence: parent_seq,
                        },
                    },
                )?;

                last_emitted_sequences.insert(node.node_id, event_record_seq);
                outcome.events_resolved += 1;

                let secondary_triggers =
                    apply_event_effect(state, &def.id.0, &node.trigger, &mut outcome)?;

                let child_depth = node.path_depth.saturating_add(def.chain_cost);

                if child_depth <= self.max_branch_depth {
                    for child_trigger in secondary_triggers {
                        next_wave.push(EventNode {
                            node_id: next_node_id,
                            parent_id: Some(node.node_id),
                            wave: current_wave + 1,
                            path_depth: child_depth,
                            parent_sequence: Some(event_record_seq),
                            trigger: child_trigger,
                        });
                        next_node_id += 1;
                    }
                }
            }

            wave_queue = next_wave;
            current_wave += 1;
        }

        Ok(outcome)
    }
}

fn apply_blizzard(state: &mut GameState) -> Result<Vec<TriggerPoint>, CoreError> {
    let from = state.weather;
    let to = Weather::Blizzard;
    if from != to {
        state.weather = to;
        crate::push_event(state, GameEvent::WeatherChanged { from, to })?;
        return Ok(vec![TriggerPoint::SecondaryTrigger {
            tag: "freeze_waters".to_owned(),
            actor_id: None,
            position: None,
        }]);
    }
    Ok(Vec::new())
}

fn apply_freeze_waters(state: &mut GameState) -> Result<(), CoreError> {
    let positions_to_freeze: Vec<(Position, usize)> = state
        .terrain
        .iter()
        .enumerate()
        .filter_map(|(idx, terrain)| {
            if *terrain == Terrain::Water {
                crate::position_from_index(idx, state.map_size)
                    .ok()
                    .map(|pos| (pos, idx))
            } else {
                None
            }
        })
        .collect();

    for (pos, idx) in positions_to_freeze {
        state.terrain[idx] = Terrain::Ice;
        crate::push_event(
            state,
            GameEvent::TerrainChanged {
                position: pos,
                from: Terrain::Water,
                to: Terrain::Ice,
            },
        )?;
    }
    Ok(())
}

fn apply_disoriented_shot(trigger: &TriggerPoint, outcome: &mut PipelineOutcome) {
    if let TriggerPoint::ActionIntent {
        command: PlayerCommand::Shoot { direction },
        ..
    } = trigger
    {
        let reversed = match direction {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        };
        outcome.override_command = Some(PlayerCommand::Shoot {
            direction: reversed,
        });
    }
}

fn apply_ice_slide(state: &mut GameState, trigger: &TriggerPoint) -> Result<(), CoreError> {
    let TriggerPoint::TerrainEntered {
        actor_id,
        to,
        direction,
        ..
    } = trigger
    else {
        return Ok(());
    };

    let Some(slide_target) = crate::step_position(*to, *direction, state.map_size) else {
        return Ok(());
    };

    let slide_idx = crate::map_index(slide_target, state.map_size)?;
    let terrain = state.terrain[slide_idx];
    let is_blocked = terrain == Terrain::Wall || terrain == Terrain::Crate;
    let has_player = crate::living_player_index_at(state, slide_target, Some(*actor_id)).is_some();

    if !is_blocked && !has_player {
        let actor_idx = crate::player_index(state, *actor_id)?;
        state.players[actor_idx].position = Some(slide_target);
        crate::push_event(
            state,
            GameEvent::Moved {
                actor_id: *actor_id,
                from: *to,
                to: slide_target,
            },
        )?;
        crate::resolve_landing_terrain(state, actor_idx, slide_target)?;
    }
    Ok(())
}

fn apply_crate_splinter_blast(state: &GameState, trigger: &TriggerPoint) -> Vec<TriggerPoint> {
    let TriggerPoint::ProjectileImpact { position, .. } = trigger else {
        return Vec::new();
    };

    let mut secondary = Vec::new();
    let pos = *position;
    let directions = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];
    for dir in directions {
        let Some(adj) = crate::step_position(pos, dir, state.map_size) else {
            continue;
        };
        if let Some(target_idx) = crate::living_player_index_at(state, adj, None) {
            let target_id = state.players[target_idx].id;
            secondary.push(TriggerPoint::SecondaryTrigger {
                tag: "crate_blast".to_owned(),
                actor_id: Some(target_id),
                position: Some(adj),
            });
        }
    }
    secondary
}

fn apply_event_effect(
    state: &mut GameState,
    event_id: &str,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<Vec<TriggerPoint>, CoreError> {
    match event_id {
        "evt_weather_blizzard" => apply_blizzard(state),
        "evt_water_freeze_ice" => {
            apply_freeze_waters(state)?;
            Ok(Vec::new())
        }
        "evt_disoriented_reverse_shot" => {
            apply_disoriented_shot(trigger, outcome);
            Ok(Vec::new())
        }
        "evt_revolver_misfire" => {
            outcome.action_canceled = true;
            Ok(Vec::new())
        }
        "evt_ice_slide" => {
            apply_ice_slide(state, trigger)?;
            Ok(Vec::new())
        }
        "evt_crate_splinter_blast" => Ok(apply_crate_splinter_blast(state, trigger)),
        _ => Ok(Vec::new()),
    }
}
