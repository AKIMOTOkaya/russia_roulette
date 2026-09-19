//! Comprehensive test suite for roulette-renderer rule-based image composition.

#![forbid(unsafe_code)]

use roulette_domain::{
    CellView, EliminationCause, EventId, EventTier, GameEvent, GameNotification, GameRecord,
    GameRecordContent, GameStatus, NotificationLevel, PlayerCommand, PlayerId, PlayerKind,
    PlayerState, PlayerStatus, Position, RefereeGameView, RefereeMemberView, RefereeRoomView,
    RoomId, RoomMemberId, RoomPhase, Terrain, Weather,
};
use roulette_renderer::{render_game_png, render_game_svg, render_match_png, render_match_svg};

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

fn mock_waiting_room() -> RefereeRoomView {
    RefereeRoomView {
        id: RoomId("ALPHA".to_string()),
        name: "战术演练场".to_string(),
        phase: RoomPhase::Waiting,
        password: None,
        referee_managed: true,
        members: vec![
            RefereeMemberView {
                id: RoomMemberId("mem-1".to_string()),
                name: "指挥官".to_string(),
                kind: PlayerKind::Human,
                is_owner: true,
                player_id: None,
                is_connected: true,
            },
            RefereeMemberView {
                id: RoomMemberId("mem-2".to_string()),
                name: "幽灵Bot 1".to_string(),
                kind: PlayerKind::Bot,
                is_owner: false,
                player_id: None,
                is_connected: true,
            },
            RefereeMemberView {
                id: RoomMemberId("mem-3".to_string()),
                name: "先锋Bot 2".to_string(),
                kind: PlayerKind::Bot,
                is_owner: false,
                player_id: None,
                is_connected: true,
            },
        ],
        game: None,
    }
}

#[allow(clippy::too_many_lines)]
fn mock_active_room() -> RefereeRoomView {
    let mut cells = Vec::new();
    // 5x5 grid with various tactical terrain
    for y in 0..5 {
        for x in 0..5 {
            let pos = Position { x, y };
            let terrain = match (x, y) {
                (1, 1) => Terrain::Wall,
                (2, 1) => Terrain::Crate,
                (3, 1) => Terrain::HighGround,
                (1, 3) => Terrain::Water,
                (2, 3) => Terrain::Ice,
                (3, 3) => Terrain::Mine,
                (4, 3) => Terrain::Medkit,
                _ => Terrain::Empty,
            };
            cells.push(CellView {
                position: pos,
                terrain,
                player_id: None,
            });
        }
    }

    let p1 = PlayerState {
        id: PlayerId(1),
        name: "指挥官".to_string(),
        kind: PlayerKind::Human,
        status: PlayerStatus::Alive,
        position: Some(Position { x: 0, y: 0 }),
        has_shield: true,
        water_turns: 0,
        consecutive_shots: 0,
    };
    let p2 = PlayerState {
        id: PlayerId(2),
        name: "幽灵Bot".to_string(),
        kind: PlayerKind::Bot,
        status: PlayerStatus::Alive,
        position: Some(Position { x: 4, y: 0 }),
        has_shield: false,
        water_turns: 0,
        consecutive_shots: 1,
    };
    let p3 = PlayerState {
        id: PlayerId(3),
        name: "先锋Bot".to_string(),
        kind: PlayerKind::Bot,
        status: PlayerStatus::Eliminated,
        position: None,
        has_shield: false,
        water_turns: 0,
        consecutive_shots: 0,
    };

    let records = vec![
        GameRecord {
            sequence: 1,
            content: GameRecordContent::Notification {
                level: NotificationLevel::Info,
                notification: GameNotification::MatchStarted { seed: 123_456 },
            },
        },
        GameRecord {
            sequence: 2,
            content: GameRecordContent::Action {
                actor_id: PlayerId(2),
                command: PlayerCommand::Shoot {
                    direction: roulette_domain::Direction::Left,
                },
            },
        },
        GameRecord {
            sequence: 3,
            content: GameRecordContent::Event {
                event: GameEvent::PlayerEliminated {
                    player_id: PlayerId(3),
                    cause: EliminationCause::Shot,
                    by_player_id: Some(PlayerId(2)),
                },
            },
        },
        GameRecord {
            sequence: 4,
            content: GameRecordContent::Event {
                event: GameEvent::DramaticEvent {
                    event_id: EventId::new("evt_ricochet"),
                    tier: EventTier::Rare,
                    title: "钢片偏转跳弹".to_string(),
                    narrative: "子弹重重擦过掩体钢板发出尖锐呼啸！".to_string(),
                    actor_id: Some(PlayerId(2)),
                    position: Some(Position { x: 1, y: 1 }),
                    wave: 1,
                    parent_sequence: Some(2),
                },
            },
        },
        GameRecord {
            sequence: 5,
            content: GameRecordContent::Turn {
                player_id: PlayerId(1),
                round: 2,
            },
        },
    ];

    let game = RefereeGameView {
        seed: 123_456,
        revision: 5,
        map_size: 5,
        cells,
        players: vec![p1, p2, p3],
        current_player_id: Some(PlayerId(1)),
        round: 2,
        status: GameStatus::Running,
        weather: Weather::Blizzard,
        alive_player_count: 2,
        records,
        event_traces: Vec::new(),
        rounds_without_elimination: 3,
    };

    RefereeRoomView {
        id: RoomId("BETA9".to_string()),
        name: "雪地绝杀".to_string(),
        phase: RoomPhase::Playing,
        password: None,
        referee_managed: true,
        members: Vec::new(),
        game: Some(game),
    }
}

#[test]
fn test_render_waiting_room_svg() {
    let room = mock_waiting_room();
    let svg = render_match_svg(&room);

    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>\n"));
    assert!(svg.contains("ALPHA"));
    assert!(svg.contains("战术演练场"));
    assert!(svg.contains("等待开局中"));
    assert!(svg.contains("幽灵Bot 1"));
    assert!(svg.contains("战术沙盘整备中"));
}

#[test]
fn test_render_waiting_room_png() {
    let room = mock_waiting_room();
    let png = render_match_png(&room).expect("failed to render waiting room PNG");

    assert!(png.len() > 1024, "PNG should be non-trivial size");
    assert_eq!(&png[0..8], &PNG_MAGIC, "PNG must have valid magic header");
}

#[test]
fn test_render_active_room_svg() {
    let room = mock_active_room();
    let svg = render_match_svg(&room);

    assert!(svg.contains("BETA9"));
    assert!(svg.contains("雪地绝杀"));
    assert!(svg.contains("激战中 · ROUND 2"));
    assert!(svg.contains("暴风雪 (水凝成冰/极滑)"));
    assert!(svg.contains("连续 3 轮未减员"));
    // Terrains (procedural gradients and hardness marks)
    assert!(svg.contains("grad-wall"));
    assert!(svg.contains("◆◆ H:2"));
    assert!(svg.contains("grad-crate"));
    assert!(svg.contains("◆ H:1"));
    assert!(svg.contains("grad-water"));
    assert!(svg.contains("grad-ice"));
    assert!(svg.contains("grad-mine"));
    assert!(svg.contains("grad-medkit"));
    // Players and indicators
    assert!(svg.contains("P1"));
    assert!(svg.contains("P2"));
    assert!(svg.contains("👉 行动中"));
    assert!(svg.contains("充能护盾"));
    // Elimination record
    assert!(svg.contains("左轮实弹击毙 (由 P2 击杀)"));
    // Drama event
    assert!(svg.contains("【钢片偏转跳弹】"));
}

#[test]
fn test_render_non_square_board() {
    use roulette_domain::{
        CellView, PlayerId, PlayerKind, PlayerState, PlayerStatus, Position, Terrain,
    };
    use roulette_renderer::{render_board_png, render_board_svg};

    // 7 columns x 4 rows non-square map
    let mut cells = Vec::new();
    for y in 0..4 {
        for x in 0..7 {
            let terrain = match (x, y) {
                (1, 1) => Terrain::Wall,
                (2, 1) => Terrain::Crate,
                (3, 2) => Terrain::Water,
                (4, 2) => Terrain::Ice,
                (5, 3) => Terrain::HighGround,
                _ => Terrain::Empty,
            };
            cells.push(CellView {
                position: Position { x, y },
                terrain,
                player_id: None,
            });
        }
    }

    let p1 = PlayerState {
        id: PlayerId(1),
        name: "Sniper P1".to_string(),
        kind: PlayerKind::Human,
        status: PlayerStatus::Alive,
        position: Some(Position { x: 0, y: 0 }),
        has_shield: true,
        water_turns: 0,
        consecutive_shots: 0,
    };
    let p2 = PlayerState {
        id: PlayerId(2),
        name: "Bot P2".to_string(),
        kind: PlayerKind::Bot,
        status: PlayerStatus::Alive,
        position: Some(Position { x: 6, y: 3 }),
        has_shield: false,
        water_turns: 0,
        consecutive_shots: 0,
    };
    let players = vec![p1, p2];

    let svg = render_board_svg(&cells, &players, Some(PlayerId(1)), &[], Some(96));
    assert!(svg.starts_with("<svg"));
    // 7 cols: 7 * 96 + 6 * 8 + 88 = 672 + 48 + 88 = 808
    // 4 rows: 4 * 96 + 3 * 8 + 88 = 384 + 24 + 88 = 496
    assert!(
        svg.contains("width=\"808\""),
        "Expected width 808 for 7 cols"
    );
    assert!(
        svg.contains("height=\"496\""),
        "Expected height 496 for 4 rows"
    );
    assert!(svg.contains("viewBox=\"0 0 808 496\""));
    // Verify coordinate rulers: columns A..G, rows 1..4
    assert!(svg.contains(">A<"));
    assert!(svg.contains(">G<"));
    assert!(svg.contains(">4<"));

    let png = render_board_png(&cells, &players, Some(PlayerId(1)), &[], Some(96))
        .expect("failed to render non-square board PNG");
    assert_eq!(&png[0..8], &PNG_MAGIC);
    assert!(png.len() > 5000);
}

#[test]
fn test_render_tilesheet_and_standalone_assets() {
    use roulette_domain::{PlayerId, PlayerKind, Terrain};
    use roulette_renderer::{
        render_single_player_svg, render_single_tile_svg, render_tilesheet_png,
        render_tilesheet_svg,
    };

    // 1. Asset tilesheet catalog
    let sheet_svg = render_tilesheet_svg();
    assert!(sheet_svg.contains("RUSSIAN ROULETTE // PROCEDURAL VECTOR ASSET CATALOG"));
    assert!(sheet_svg.contains("TERRAIN TILES"));
    assert!(sheet_svg.contains("COMBATANT TOKENS"));
    assert!(sheet_svg.contains("BALLISTIC LASER"));

    let sheet_png = render_tilesheet_png().expect("failed to render tilesheet PNG");
    assert_eq!(&sheet_png[0..8], &PNG_MAGIC);
    assert!(sheet_png.len() > 10_000);

    // 2. Standalone single tile
    let wall_svg = render_single_tile_svg(Terrain::Wall, 96);
    assert!(wall_svg.contains("grad-wall"));
    assert!(wall_svg.contains("◆◆ H:2"));

    // 3. Standalone player token
    let player_svg = render_single_player_svg(PlayerId(1), PlayerKind::Human, true, true, 80);
    assert!(player_svg.contains("Shield Barrier"));
    assert!(player_svg.contains("Turn Indicator"));
    assert!(player_svg.contains("P1"));
}

#[test]
fn test_render_active_room_png() {
    let room = mock_active_room();
    let png = render_match_png(&room).expect("failed to render active match PNG");

    assert!(
        png.len() > 10_000,
        "Active match PNG should be at least 10KB"
    );
    assert_eq!(&png[0..8], &PNG_MAGIC, "PNG must have valid magic header");
}

#[test]
fn test_render_finished_room_with_winner() {
    let mut room = mock_active_room();
    if let Some(game) = &mut room.game {
        game.status = GameStatus::Finished {
            winner_id: Some(PlayerId(1)),
        };
    }
    room.phase = RoomPhase::Finished;

    let svg = render_match_svg(&room);
    assert!(svg.contains("胜者加冕: PLAYER 1 最终幸存！"));

    let png = render_match_png(&room).expect("failed to render finished match PNG");
    assert_eq!(&png[0..8], &PNG_MAGIC);
}

#[test]
fn test_render_game_view() {
    let room = mock_active_room();
    let referee_game = room.game.unwrap();
    let game_view = roulette_domain::GameView {
        seed: referee_game.seed,
        revision: referee_game.revision,
        map_size: referee_game.map_size,
        cells: referee_game.cells,
        players: referee_game.players,
        current_player_id: referee_game.current_player_id,
        human_player_id: PlayerId(1),
        round: referee_game.round,
        status: referee_game.status,
        weather: referee_game.weather,
        records: referee_game.records,
        event_traces: referee_game.event_traces,
        rounds_without_elimination: referee_game.rounds_without_elimination,
    };

    let svg = render_game_svg(&game_view, "普通玩家房", "USR01");
    assert!(svg.contains("普通玩家房"));
    assert!(svg.contains("USR01"));

    let png =
        render_game_png(&game_view, "普通玩家房", "USR01").expect("failed to render game view PNG");
    assert_eq!(&png[0..8], &PNG_MAGIC);
}
