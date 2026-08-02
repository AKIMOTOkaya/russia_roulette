//! Stable domain contracts for Russian Roulette.
//!
//! The domain crate contains serializable data shared by the rules engine,
//! local host, HTTP adapter, tests, and future clients. It has no knowledge of
//! networking, databases, files, clocks, or UI frameworks.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Stable identifier for a player within one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerId(pub u32);

/// A coordinate on the square map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// Horizontal coordinate, increasing to the right.
    pub x: u8,
    /// Vertical coordinate, increasing downward.
    pub y: u8,
}

/// Four-way movement and shooting direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Move or shoot upward.
    Up,
    /// Move or shoot downward.
    Down,
    /// Move or shoot left.
    Left,
    /// Move or shoot right.
    Right,
}

/// Terrain types enabled by the local Web MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    /// Walkable ground without an effect.
    Empty,
    /// Impassable and bullet-blocking wall.
    Wall,
    /// Destructible bullet-blocking crate.
    Crate,
    /// Walkable water that drowns a player after two consecutive turns.
    Water,
    /// Walkable high ground that grants one additional shooting tile.
    HighGround,
    /// Walkable mine that triggers a lethal effect and then disappears.
    Mine,
    /// Walkable one-use pickup that grants a single lethal shield.
    Medkit,
}

/// Whether a player is controlled by a person or the local random bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKind {
    /// Human player using the browser.
    Human,
    /// In-process random bot.
    Bot,
}

/// Player life-cycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerStatus {
    /// Player may still take turns.
    Alive,
    /// Player has been removed from the map and turn order.
    Eliminated,
}

/// Player information stored in the authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// Match-local player identifier.
    pub id: PlayerId,
    /// Display name.
    pub name: String,
    /// Human or bot controller.
    pub kind: PlayerKind,
    /// Current life-cycle status.
    pub status: PlayerStatus,
    /// Current map position; eliminated players have no position.
    pub position: Option<Position>,
    /// Whether the one-use lethal shield is available.
    pub has_shield: bool,
    /// Consecutive turns ended in water.
    pub water_turns: u8,
    /// Consecutive shooting actions since the last non-shooting action.
    pub consecutive_shots: u8,
}

/// Player definition used when creating a match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSetup {
    /// Display name.
    pub name: String,
    /// Human or bot controller.
    pub kind: PlayerKind,
}

/// Explicit inputs required to create a deterministic game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGameConfig {
    /// Seed from which independent random streams are derived.
    pub seed: u64,
    /// Ordered players; the first player takes the first turn.
    pub players: Vec<PlayerSetup>,
}

/// Commands accepted by the MVP rules engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerCommand {
    /// Move one tile in a cardinal direction.
    Move {
        /// Requested direction.
        direction: Direction,
    },
    /// Shoot the revolver in a cardinal direction.
    Shoot {
        /// Requested direction.
        direction: Direction,
    },
    /// Consume the turn without moving or shooting.
    Wait,
    /// Immediately eliminate the acting player.
    Suicide,
}

/// Why a movement command could not enter its target tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    /// Target lies outside the map.
    Boundary,
    /// Target contains a wall.
    Wall,
    /// Target contains a crate.
    Crate,
}

/// Cause recorded when a player is eliminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EliminationCause {
    /// Hit by a revolver bullet.
    Shot,
    /// Lost an elbow duel after sharing a tile.
    ElbowDuel,
    /// Triggered a mine.
    Mine,
    /// Ended two consecutive turns in water.
    Drowned,
    /// Chose the suicide action.
    Suicide,
}

/// Structured facts emitted by the rules engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    /// Initial state was created.
    MatchStarted {
        /// Seed used for this match.
        seed: u64,
    },
    /// A player entered another tile.
    Moved {
        /// Acting player.
        actor_id: PlayerId,
        /// Previous position.
        from: Position,
        /// New position.
        to: Position,
    },
    /// A movement command was blocked but still consumed the turn.
    MoveBlocked {
        /// Acting player.
        actor_id: PlayerId,
        /// Why movement was blocked.
        reason: BlockReason,
    },
    /// Two players met in one tile and immediately resolved a duel.
    ElbowDuel {
        /// Player who entered the occupied tile.
        attacker_id: PlayerId,
        /// Player who already occupied the tile.
        defender_id: PlayerId,
        /// Surviving player.
        winner_id: PlayerId,
    },
    /// A bullet left the revolver.
    ShotFired {
        /// Acting player.
        actor_id: PlayerId,
        /// Firing direction.
        direction: Direction,
    },
    /// The revolver clicked without firing.
    EmptyChamber {
        /// Acting player.
        actor_id: PlayerId,
        /// Whether this was the guaranteed third consecutive empty shot.
        forced: bool,
    },
    /// A fired bullet hit neither a player nor a crate.
    ShotMissed {
        /// Acting player.
        actor_id: PlayerId,
    },
    /// A lethal effect consumed a player's one-use shield.
    ShieldConsumed {
        /// Protected player.
        player_id: PlayerId,
        /// Effect that would otherwise have eliminated the player.
        cause: EliminationCause,
    },
    /// A player was removed from the match.
    PlayerEliminated {
        /// Eliminated player.
        player_id: PlayerId,
        /// Elimination cause.
        cause: EliminationCause,
        /// Responsible player, when applicable.
        by_player_id: Option<PlayerId>,
    },
    /// A terrain tile changed.
    TerrainChanged {
        /// Changed tile.
        position: Position,
        /// Previous terrain.
        from: Terrain,
        /// New terrain.
        to: Terrain,
    },
    /// A player collected a one-use item.
    ItemCollected {
        /// Collecting player.
        player_id: PlayerId,
        /// Item tile consumed by the pickup.
        item: Terrain,
    },
    /// A player intentionally waited.
    Waited {
        /// Acting player.
        actor_id: PlayerId,
    },
    /// A new player turn began.
    TurnStarted {
        /// Current player.
        player_id: PlayerId,
        /// One-based round number.
        round: u32,
    },
    /// The match reached a terminal state.
    GameFinished {
        /// Last surviving player, or none for a draw.
        winner_id: Option<PlayerId>,
    },
}

/// Terminal or running status of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum GameStatus {
    /// Commands may still be submitted.
    Running,
    /// No further commands are accepted.
    Finished {
        /// Last surviving player, or none for a draw.
        winner_id: Option<PlayerId>,
    },
}

/// Independent deterministic random streams stored with the game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngStreams {
    /// Map generation stream.
    pub map: u64,
    /// Collision and encounter stream.
    pub encounter: u64,
    /// Revolver and combat stream.
    pub combat: u64,
    /// Local bot decision stream.
    pub bot: u64,
}

/// Complete authoritative game state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    /// Creation seed.
    pub seed: u64,
    /// Monotonically increasing command revision.
    pub revision: u64,
    /// Square map edge length.
    pub map_size: u8,
    /// Row-major terrain cells.
    pub terrain: Vec<Terrain>,
    /// All players, including eliminated players.
    pub players: Vec<PlayerState>,
    /// Stable player turn order.
    pub turn_order: Vec<PlayerId>,
    /// Index into `turn_order` for the current player.
    pub current_turn_index: usize,
    /// One-based round counter.
    pub round: u32,
    /// Running or finished state.
    pub status: GameStatus,
    /// Explicit random streams required for replay.
    pub rng: RngStreams,
    /// Chronological structured event log.
    pub events: Vec<GameEvent>,
}

/// Public map cell used by the Web client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellView {
    /// Cell position.
    pub position: Position,
    /// Visible terrain.
    pub terrain: Terrain,
    /// Living player occupying the cell, if any.
    pub player_id: Option<PlayerId>,
}

/// Public snapshot returned to the local Web client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameView {
    /// Creation seed.
    pub seed: u64,
    /// Current authoritative revision.
    pub revision: u64,
    /// Square map edge length.
    pub map_size: u8,
    /// Row-major visible cells.
    pub cells: Vec<CellView>,
    /// Public player information.
    pub players: Vec<PlayerState>,
    /// Player expected to act, when the game is running.
    pub current_player_id: Option<PlayerId>,
    /// Human player controlled by this browser.
    pub human_player_id: PlayerId,
    /// One-based round counter.
    pub round: u32,
    /// Running or finished state.
    pub status: GameStatus,
    /// Recent chronological structured events.
    pub events: Vec<GameEvent>,
}
