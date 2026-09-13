//! Breadth-First-Search event tree pipeline.

use std::collections::HashMap;

use roulette_domain::{
    Direction, EventNodeTrace, GameEvent, GameRecordContent, GameState, PipelineTrace,
    PlayerCommand, PlayerStatus, Position, Terrain, Weather,
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
#[allow(clippy::struct_excessive_bools)]
pub struct PipelineOutcome {
    /// Command override if an `ActionIntent` event altered direction or behavior.
    pub override_command: Option<PlayerCommand>,
    /// Whether the action was canceled (e.g. misfire, stumble, spatial swap).
    pub action_canceled: bool,
    /// Total dramatic events emitted into the record log.
    pub events_resolved: usize,
    /// Recoil knockback direction to apply after shooting.
    pub recoil_direction: Option<Direction>,
    /// Whether this shot penetrates obstacles.
    pub piercing_shot: bool,
    /// Whether this shot ricochets towards the nearest living target.
    pub deadly_ricochet: bool,
    /// Whether this move triggers an extra sprint step.
    pub sprint_dash: bool,
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
    #[allow(clippy::too_many_lines)]
    pub fn run(
        &self,
        state: &mut GameState,
        root_trigger: TriggerPoint,
    ) -> Result<PipelineOutcome, CoreError> {
        let mut outcome = PipelineOutcome::default();
        let root_desc = root_trigger.describe();
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
        let mut node_traces: Vec<EventNodeTrace> = Vec::new();

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

                let (chosen, total_weight, roll_value, candidates) = pool.roll_with_trace(
                    node.path_depth,
                    state.rounds_without_elimination,
                    &mut state.rng.events,
                );

                let Some(event_id) = chosen else {
                    continue;
                };

                let trigger_desc = node.trigger.describe();
                let pool_name = pool.name.to_string();

                // Silent no-op dampener: closes the branch without creating a record
                if event_id.0 == "evt_nothing_happens" {
                    node_traces.push(EventNodeTrace {
                        node_id: node.node_id,
                        parent_id: node.parent_id,
                        wave: node.wave,
                        path_depth: node.path_depth,
                        trigger_desc,
                        pool_name,
                        total_weight,
                        roll_value,
                        selected_event_id: Some(event_id),
                        selected_title: Some("风平浪静".to_string()),
                        candidates,
                        outcome_desc: "无事发生，分支平息收敛".to_string(),
                    });
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

                let (secondary_triggers, outcome_desc) =
                    apply_event_effect(state, &def.id.0, &node.trigger, &mut outcome)?;

                node_traces.push(EventNodeTrace {
                    node_id: node.node_id,
                    parent_id: node.parent_id,
                    wave: node.wave,
                    path_depth: node.path_depth,
                    trigger_desc,
                    pool_name,
                    total_weight,
                    roll_value,
                    selected_event_id: Some(def.id.clone()),
                    selected_title: Some(def.title.to_string()),
                    candidates,
                    outcome_desc,
                });

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

        if !node_traces.is_empty() {
            let trace_id = state
                .event_traces
                .last()
                .map_or(1, |t| t.id.saturating_add(1));
            let pipeline_trace = PipelineTrace {
                id: trace_id,
                round: state.round,
                revision: state.revision,
                stalemate_rounds: state.rounds_without_elimination,
                root_trigger_desc: root_desc,
                nodes: node_traces,
            };
            state.event_traces.push(pipeline_trace);
            if state.event_traces.len() > 30 {
                let excess = state.event_traces.len() - 30;
                state.event_traces.drain(0..excess);
            }
        }

        Ok(outcome)
    }
}

fn apply_blizzard(state: &mut GameState) -> Result<(Vec<TriggerPoint>, String), CoreError> {
    let from = state.weather;
    let to = Weather::Blizzard;
    if from != to {
        state.weather = to;
        crate::push_event(state, GameEvent::WeatherChanged { from, to })?;
        return Ok((
            vec![TriggerPoint::SecondaryTrigger {
                tag: "freeze_waters".to_owned(),
                actor_id: None,
                position: None,
            }],
            "刺骨暴雪骤降，全图气温骤降并引发水面凝结连锁".to_string(),
        ));
    }
    Ok((Vec::new(), "暴雪持续呼啸".to_string()))
}

fn apply_freeze_waters(state: &mut GameState) -> Result<(Vec<TriggerPoint>, String), CoreError> {
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

    let count = positions_to_freeze.len();
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
    Ok((Vec::new(), format!("寒气封冻了全图 {count} 处水面为坚冰")))
}

fn apply_heatwave(state: &mut GameState) -> Result<(Vec<TriggerPoint>, String), CoreError> {
    let from = state.weather;
    let to = Weather::Heatwave;
    if from != to {
        state.weather = to;
        crate::push_event(state, GameEvent::WeatherChanged { from, to })?;
    }

    let positions_to_melt: Vec<(Position, usize)> = state
        .terrain
        .iter()
        .enumerate()
        .filter_map(|(idx, terrain)| {
            if *terrain == Terrain::Ice {
                crate::position_from_index(idx, state.map_size)
                    .ok()
                    .map(|pos| (pos, idx))
            } else {
                None
            }
        })
        .collect();

    let count = positions_to_melt.len();
    for (pos, idx) in positions_to_melt {
        state.terrain[idx] = Terrain::Water;
        crate::push_event(
            state,
            GameEvent::TerrainChanged {
                position: pos,
                from: Terrain::Ice,
                to: Terrain::Water,
            },
        )?;
    }
    Ok((
        Vec::new(),
        format!("热浪席卷战场，融化了全图 {count} 处坚冰为水面"),
    ))
}

fn apply_meteor_strike(state: &mut GameState) -> Result<(Vec<TriggerPoint>, String), CoreError> {
    let strike_idx = crate::random_index(&mut state.rng.events, state.terrain.len())?;
    let strike_pos = crate::position_from_index(strike_idx, state.map_size)?;
    let old_terrain = state.terrain[strike_idx];
    let new_terrain = if old_terrain == Terrain::Empty {
        Terrain::Water
    } else {
        Terrain::Empty
    };
    state.terrain[strike_idx] = new_terrain;
    crate::push_event(
        state,
        GameEvent::TerrainChanged {
            position: strike_pos,
            from: old_terrain,
            to: new_terrain,
        },
    )?;

    let mut struck_player = None;
    if let Some(target_idx) = crate::living_player_index_at(state, strike_pos, None) {
        let pid = state.players[target_idx].id;
        struck_player = Some(pid);
        crate::eliminate_player(
            state,
            target_idx,
            roulette_domain::EliminationCause::Shot,
            None,
            false,
        )?;
    }

    let victim_str = struck_player
        .map(|p| format!("，直接轰杀淘汰了玩家 P{}", p.0))
        .unwrap_or_default();
    Ok((
        Vec::new(),
        format!(
            "陨石重击地表 ({}, {})，地形由 {:?} 变为 {:?}{}",
            strike_pos.x, strike_pos.y, old_terrain, new_terrain, victim_str
        ),
    ))
}

fn apply_disoriented_shot(trigger: &TriggerPoint, outcome: &mut PipelineOutcome) -> String {
    if let TriggerPoint::ActionIntent {
        command: PlayerCommand::Shoot { direction },
        ..
    } = trigger
    {
        let reversed = direction.opposite();
        outcome.override_command = Some(PlayerCommand::Shoot {
            direction: reversed,
        });
        format!("射手视野昏花，开枪方向由 {direction:?} 反转为 {reversed:?}")
    } else {
        "开枪方向发生偏转".to_string()
    }
}

fn apply_recoil_knockback(trigger: &TriggerPoint, outcome: &mut PipelineOutcome) -> String {
    if let TriggerPoint::ActionIntent {
        command: PlayerCommand::Shoot { direction },
        ..
    } = trigger
    {
        let recoil_dir = direction.opposite();
        outcome.recoil_direction = Some(recoil_dir);
        format!("转轮超量装药！射手将承受向 {recoil_dir:?} 的强力后坐力反冲")
    } else {
        "产生后坐力反冲".to_string()
    }
}

fn apply_spatial_swap(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    outcome.action_canceled = true;
    let Some(actor_id) = trigger.actor_id() else {
        return Ok("空间对调触发，无有效行动者".to_string());
    };

    let other_indices: Vec<usize> = state
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.status == PlayerStatus::Alive && p.id != actor_id)
        .map(|(idx, _)| idx)
        .collect();

    if other_indices.is_empty() {
        return Ok("空间对调紊乱，但未检测到其他活体目标".to_string());
    }

    let pick = crate::random_index(&mut state.rng.events, other_indices.len())?;
    let other_idx = other_indices[pick];
    let actor_idx = crate::player_index(state, actor_id)?;

    let pos_a = state.players[actor_idx]
        .position
        .ok_or(CoreError::InvalidState("actor has no position"))?;
    let pos_b = state.players[other_idx]
        .position
        .ok_or(CoreError::InvalidState("other player has no position"))?;

    state.players[actor_idx].position = Some(pos_b);
    state.players[other_idx].position = Some(pos_a);

    let other_id = state.players[other_idx].id;
    crate::push_event(
        state,
        GameEvent::Moved {
            actor_id,
            from: pos_a,
            to: pos_b,
        },
    )?;
    crate::push_event(
        state,
        GameEvent::Moved {
            actor_id: other_id,
            from: pos_b,
            to: pos_a,
        },
    )?;

    crate::resolve_landing_terrain(state, actor_idx, pos_b)?;
    crate::resolve_landing_terrain(state, other_idx, pos_a)?;

    Ok(format!(
        "地磁剧烈混乱！玩家 P{} 与 P{} 瞬间空间互换位置 ({}, {}) <=> ({}, {})",
        actor_id.0, other_id.0, pos_a.x, pos_a.y, pos_b.x, pos_b.y
    ))
}

fn apply_ice_crack_collapse(
    state: &mut GameState,
    trigger: &TriggerPoint,
) -> Result<String, CoreError> {
    let TriggerPoint::TerrainEntered { actor_id, to, .. } = trigger else {
        return Ok("薄冰碎裂".to_string());
    };

    let idx = crate::map_index(*to, state.map_size)?;
    state.terrain[idx] = Terrain::Water;
    crate::push_event(
        state,
        GameEvent::TerrainChanged {
            position: *to,
            from: Terrain::Ice,
            to: Terrain::Water,
        },
    )?;

    let actor_idx = crate::player_index(state, *actor_id)?;
    state.players[actor_idx].water_turns = 1;

    Ok(format!(
        "薄冰碎裂崩塌！({}, {}) 化为深水，玩家 P{} 猝不及防落入水中",
        to.x, to.y, actor_id.0
    ))
}

fn apply_crate_surprise(
    state: &mut GameState,
    trigger: &TriggerPoint,
    is_mine: bool,
) -> Result<String, CoreError> {
    let TriggerPoint::ProjectileImpact { position, .. } = trigger else {
        return Ok("木箱掉落物揭示".to_string());
    };

    let idx = crate::map_index(*position, state.map_size)?;
    let (new_terrain, item_name) = if is_mine {
        (Terrain::Mine, "现役触发式地雷")
    } else {
        (Terrain::Medkit, "防护单兵盾")
    };

    state.terrain[idx] = new_terrain;
    crate::push_event(
        state,
        GameEvent::TerrainChanged {
            position: *position,
            from: Terrain::Empty,
            to: new_terrain,
        },
    )?;

    Ok(format!(
        "木箱爆碎后在 ({}, {}) 掉落暴露出一件 {}",
        position.x, position.y, item_name
    ))
}

fn apply_ice_slide(
    state: &mut GameState,
    trigger: &TriggerPoint,
) -> Result<(Vec<TriggerPoint>, String), CoreError> {
    let TriggerPoint::TerrainEntered {
        actor_id,
        to,
        direction,
        ..
    } = trigger
    else {
        return Ok((Vec::new(), "冰面滑行未生效".to_string()));
    };

    let Some(slide_target) = crate::step_position(*to, *direction, state.map_size) else {
        return Ok((Vec::new(), "滑行被边缘墙体阻挡".to_string()));
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
        Ok((
            Vec::new(),
            format!(
                "玩家 P{} 踏上光滑冰面向前滑行至 ({}, {})",
                actor_id.0, slide_target.x, slide_target.y
            ),
        ))
    } else {
        Ok((Vec::new(), "滑行前方受阻，停在当前冰格".to_string()))
    }
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

pub(crate) fn apply_sudden_landmine(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    if let TriggerPoint::ActionIntent { actor_id, .. } = trigger {
        outcome.action_canceled = true;
        let actor_idx = crate::player_index(state, *actor_id)?;
        let eliminated = crate::eliminate_player(
            state,
            actor_idx,
            roulette_domain::EliminationCause::Mine,
            None,
            true,
        )?;
        if eliminated {
            Ok(format!(
                "暗雷轰然引爆！玩家 P{} 当场触雷被炸飞出局！",
                actor_id.0
            ))
        } else {
            Ok(format!(
                "暗雷轰然引爆！玩家 P{} 的护盾抵御了爆炸冲击并破裂！",
                actor_id.0
            ))
        }
    } else {
        Ok("隐蔽暗雷轰然引爆".to_string())
    }
}

fn apply_wall_fatal_concussion(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    outcome.action_canceled = true;
    if let TriggerPoint::ActionIntent { actor_id, .. } = trigger {
        let actor_idx = crate::player_index(state, *actor_id)?;
        let eliminated = crate::eliminate_player(
            state,
            actor_idx,
            roulette_domain::EliminationCause::Collision,
            None,
            true,
        )?;
        if eliminated {
            Ok(format!(
                "玩家 P{} 全速撞击坚硬墙体，颅骨碎裂当场毙命！",
                actor_id.0
            ))
        } else {
            Ok(format!(
                "玩家 P{} 猛烈撞击墙体，护盾吸收了致命冲击并破碎！",
                actor_id.0
            ))
        }
    } else {
        Ok("全速撞击坚硬墙体，颅骨碎裂".to_string())
    }
}

fn apply_wall_collapse_crush(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    outcome.action_canceled = true;
    if let TriggerPoint::ActionIntent { actor_id, .. } = trigger {
        let actor_idx = crate::player_index(state, *actor_id)?;
        let eliminated = crate::eliminate_player(
            state,
            actor_idx,
            roulette_domain::EliminationCause::Collision,
            None,
            true,
        )?;
        if eliminated {
            Ok(format!(
                "墙体承重坍塌，坠落的沉重巨石当场将玩家 P{} 砸毙！",
                actor_id.0
            ))
        } else {
            Ok(format!(
                "沉重巨石砸落，玩家 P{} 的护盾承受了重击并破碎！",
                actor_id.0
            ))
        }
    } else {
        Ok("墙体承重坍塌，沉重巨石砸落".to_string())
    }
}

fn apply_wall_rebound_detonation(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    outcome.action_canceled = true;
    if let TriggerPoint::ActionIntent { actor_id, .. } = trigger {
        let actor_idx = crate::player_index(state, *actor_id)?;
        let eliminated = crate::eliminate_player(
            state,
            actor_idx,
            roulette_domain::EliminationCause::Mine,
            None,
            true,
        )?;
        if eliminated {
            Ok(format!(
                "玩家 P{} 撞墙失控后仰摔在隐蔽暗雷上，引爆当场身亡！",
                actor_id.0
            ))
        } else {
            Ok(format!(
                "撞墙反弹触雷引爆，玩家 P{} 的护盾抵挡了爆炸冲击并破碎！",
                actor_id.0
            ))
        }
    } else {
        Ok("撞墙反弹触雷引爆".to_string())
    }
}

fn apply_wall_stun_survive(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    outcome.action_canceled = true;
    if let TriggerPoint::ActionIntent {
        actor_id,
        command: PlayerCommand::Move { direction },
    } = trigger
    {
        let actor_idx = crate::player_index(state, *actor_id)?;
        let pos = state.players[actor_idx].position;
        let target = pos.and_then(|p| crate::step_position(p, *direction, state.map_size));
        let reason = if target.is_none() {
            roulette_domain::BlockReason::Boundary
        } else {
            roulette_domain::BlockReason::Wall
        };
        crate::push_event(
            state,
            GameEvent::MoveBlocked {
                actor_id: *actor_id,
                reason,
            },
        )?;
        Ok(format!(
            "玩家 P{} 撞墙头晕目眩，但万幸没有伤及性命！",
            actor_id.0
        ))
    } else {
        Ok("撞墙头晕目眩，侥幸保住一命".to_string())
    }
}

fn apply_wall_breakthrough(
    state: &mut GameState,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<String, CoreError> {
    outcome.action_canceled = true;
    if let TriggerPoint::ActionIntent {
        actor_id,
        command: PlayerCommand::Move { direction },
    } = trigger
    {
        let actor_idx = crate::player_index(state, *actor_id)?;
        let from = state.players[actor_idx].position;
        let target = from.and_then(|p| crate::step_position(p, *direction, state.map_size));
        if let (Some(from_pos), Some(target_pos)) = (from, target) {
            let t_idx = crate::map_index(target_pos, state.map_size)?;
            if state.terrain[t_idx] == Terrain::Wall {
                state.terrain[t_idx] = Terrain::Empty;
                crate::push_event(
                    state,
                    GameEvent::TerrainChanged {
                        position: target_pos,
                        from: Terrain::Wall,
                        to: Terrain::Empty,
                    },
                )?;
                crate::complete_move_step(state, actor_idx, from_pos, target_pos, *direction)?;
                return Ok(format!(
                    "蛮力冲撞！玩家 P{} 竟硬生生撞塌了墙体并破墙冲入！",
                    actor_id.0
                ));
            }
        }
        crate::push_event(
            state,
            GameEvent::MoveBlocked {
                actor_id: *actor_id,
                reason: roulette_domain::BlockReason::Boundary,
            },
        )?;
        Ok(format!(
            "玩家 P{} 猛烈冲撞边界，硬生生停下了脚步。",
            actor_id.0
        ))
    } else {
        Ok("蛮力冲撞撞塌了墙体".to_string())
    }
}

fn apply_event_effect(
    state: &mut GameState,
    event_id: &str,
    trigger: &TriggerPoint,
    outcome: &mut PipelineOutcome,
) -> Result<(Vec<TriggerPoint>, String), CoreError> {
    match event_id {
        "evt_weather_blizzard" => apply_blizzard(state),
        "evt_water_freeze_ice" => apply_freeze_waters(state),
        "evt_weather_heatwave" => apply_heatwave(state),
        "evt_meteor_strike" => apply_meteor_strike(state),
        "evt_disoriented_reverse_shot" => {
            let desc = apply_disoriented_shot(trigger, outcome);
            Ok((Vec::new(), desc))
        }
        "evt_revolver_misfire" => {
            outcome.action_canceled = true;
            Ok((Vec::new(), "转轮哑火撞针空响，开火行动无效".to_string()))
        }
        "evt_recoil_knockback" => {
            let desc = apply_recoil_knockback(trigger, outcome);
            Ok((Vec::new(), desc))
        }
        "evt_piercing_slug" => {
            outcome.piercing_shot = true;
            Ok((
                Vec::new(),
                "装填穿甲重弹，弹头将击碎并穿透木箱掩体".to_string(),
            ))
        }
        "evt_ricochet_deadly" => {
            outcome.deadly_ricochet = true;
            Ok((
                Vec::new(),
                "致命跳弹附魔：子弹在偏折后将自动折射扑向最近存活玩家".to_string(),
            ))
        }
        "evt_sprint_dash" => {
            let desc = "脚步发力过猛骤然突进，移动距离额外增加一格".to_string();
            outcome.sprint_dash = true;
            Ok((Vec::new(), desc))
        }
        "evt_stumble_trip" => {
            outcome.action_canceled = true;
            let desc = "脚底绊蒜摔倒在地，移动行动被取消并浪费回合".to_string();
            Ok((Vec::new(), desc))
        }
        "evt_spatial_swap" => {
            let desc = apply_spatial_swap(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_sudden_landmine" => {
            let desc = apply_sudden_landmine(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_wall_fatal_concussion" => {
            let desc = apply_wall_fatal_concussion(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_wall_collapse_crush" => {
            let desc = apply_wall_collapse_crush(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_wall_rebound_detonation" => {
            let desc = apply_wall_rebound_detonation(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_wall_stun_survive" => {
            let desc = apply_wall_stun_survive(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_wall_breakthrough" => {
            let desc = apply_wall_breakthrough(state, trigger, outcome)?;
            Ok((Vec::new(), desc))
        }
        "evt_ice_slide" => apply_ice_slide(state, trigger),
        "evt_ice_crack_collapse" => {
            let desc = apply_ice_crack_collapse(state, trigger)?;
            Ok((Vec::new(), desc))
        }
        "evt_crate_surprise_mine" => {
            let desc = apply_crate_surprise(state, trigger, true)?;
            Ok((Vec::new(), desc))
        }
        "evt_crate_surprise_medkit" => {
            let desc = apply_crate_surprise(state, trigger, false)?;
            Ok((Vec::new(), desc))
        }
        "evt_crate_splinter_blast" => {
            let secondary = apply_crate_splinter_blast(state, trigger);
            let desc = format!("木箱破裂四向飞溅，波及周围 {} 处目标", secondary.len());
            Ok((secondary, desc))
        }
        "evt_splinter_scratch" => Ok((Vec::new(), "飞屑划伤目标并造成微小冲击".to_string())),
        "evt_dust_settles" => Ok((Vec::new(), "激荡的冲击波消散，烟尘落定".to_string())),
        _ => Ok((Vec::new(), "事件已结算".to_string())),
    }
}
