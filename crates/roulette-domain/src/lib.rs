//! Stable domain contracts for Russian Roulette.
//!
//! The domain crate contains serializable data shared by the rules engine,
//! local host, HTTP adapter, tests, and future clients. It has no knowledge of
//! networking, databases, files, clocks, or UI frameworks.

#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable identifier for a player within one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PlayerId(pub u32);

/// Browser-tab identity used by the local lobby.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct TabId(pub String);

/// Case-insensitive five-character room identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RoomId(pub String);

/// Stable room-member identifier for either a human tab or a bot slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RoomMemberId(pub String);

/// Stable identifier indexing an event definition in the event catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EventId(pub String);

impl EventId {
    /// Creates a static or owned event identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Rarity and dramatic impact tier of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventTier {
    /// Common, ordinary events (~10%).
    #[default]
    Normal,
    /// Minor unexpected events (~40%).
    Uncommon,
    /// Game-altering events (~30%).
    Rare,
    /// High-impact epic events (~15%).
    Epic,
    /// Extreme legendary/dramatic events (~5%).
    Legendary,
}

/// Detailed diagnostic record of an event candidate during a dynamic pool roll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CandidateTrace {
    /// Event identifier.
    pub event_id: EventId,
    /// Human-readable event title.
    pub title: String,
    /// Base selection weight.
    pub base_weight: u32,
    /// Effective weight after branch depth dampening.
    pub effective_weight: u64,
    /// Probability permille (e.g. 450 = 45.0%).
    pub probability_permille: u32,
    /// Whether this entry serves as a chain dampener.
    pub is_dampener: bool,
    /// Whether this entry is lethal and scales up with rounds without elimination.
    pub is_lethal: bool,
    /// Whether this candidate was selected by the roll.
    pub selected: bool,
}

/// Detailed diagnostic trace of a single node in the event BFS resolution tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EventNodeTrace {
    /// Node sequence identifier in the pipeline run.
    pub node_id: u32,
    /// Parent node ID (None for root wave 0).
    pub parent_id: Option<u32>,
    /// Chronological BFS wave (0 is root).
    pub wave: u32,
    /// Cumulative path depth along this tree branch.
    pub path_depth: u32,
    /// Human-readable trigger context description.
    pub trigger_desc: String,
    /// Diagnostic event pool name evaluated.
    pub pool_name: String,
    /// Sum of all candidates' effective weights.
    pub total_weight: u64,
    /// Pseudo-random roll value drawn from events RNG.
    pub roll_value: u64,
    /// Selected event ID, if any.
    pub selected_event_id: Option<EventId>,
    /// Selected event title, if any.
    pub selected_title: Option<String>,
    /// All candidates evaluated in this pool roll.
    pub candidates: Vec<CandidateTrace>,
    /// Actionable outcome or state change performed by this node.
    pub outcome_desc: String,
}

/// Complete diagnostic trace of a single BFS event pipeline execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineTrace {
    /// Monotonic run identifier.
    pub id: u64,
    /// Match round when triggered.
    pub round: u32,
    /// Command revision during this execution.
    pub revision: u64,
    /// Consecutive rounds without any player elimination during this execution.
    pub stalemate_rounds: u32,
    /// Summary of the root trigger point.
    pub root_trigger_desc: String,
    /// All tree nodes executed in BFS wave order.
    pub nodes: Vec<EventNodeTrace>,
}

/// Global weather and environmental condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Weather {
    /// Normal clear conditions without global effect.
    #[default]
    Clear,
    /// Severe blizzard that freezes water into ice and increases sliding chance.
    Blizzard,
    /// Oppressive heatwave.
    Heatwave,
    /// Dense fog.
    DenseFog,
}

/// A coordinate on the square map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    /// Horizontal coordinate, increasing to the right.
    pub x: u8,
    /// Vertical coordinate, increasing downward.
    pub y: u8,
}

/// Four-way movement and shooting direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

impl Direction {
    /// Returns the inverted opposite direction.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// Terrain types enabled by the local Web MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Walkable slippery ice that alters movement and event pools.
    Ice,
}

/// Placement layer or elevation of a terrain type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerrainLayer {
    /// Above the ground (e.g. wall, crate, ice, empty).
    AboveGround,
    /// Below ground surface (e.g. buried landmine).
    Underground,
    /// Both above and below ground (e.g. deep water basin).
    AboveAndBelow,
    /// Special tactical or item elevation.
    Special,
}

impl TerrainLayer {
    /// Human-readable Chinese label for the placement layer.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AboveGround => "地上",
            Self::Underground => "地下",
            Self::AboveAndBelow => "地上且地下",
            Self::Special => "特殊",
        }
    }
}

/// Static mechanical attributes and destruction properties of a terrain type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TerrainProperties {
    /// Associated terrain enum.
    pub terrain: Terrain,
    /// Chinese display name.
    pub name: String,
    /// Map symbol.
    pub symbol: String,
    /// Layer position: 地上 / 地下 / 地上且地下 / 特殊.
    pub layer: TerrainLayer,
    /// Hardness rating (e.g. 1 for crate, 2 for wall) or None.
    pub hardness: Option<u32>,
    /// Target terrain when destroyed, or None if indestructible.
    pub transform_on_destroy: Option<Terrain>,
    /// Detailed description and tactical interaction rules.
    pub description: String,
}

impl Terrain {
    /// Returns the static mechanical and tactical properties for this terrain type.
    #[must_use]
    pub fn properties(self) -> TerrainProperties {
        match self {
            Self::Empty => TerrainProperties {
                terrain: Self::Empty,
                name: "空地".to_string(),
                symbol: String::new(),
                layer: TerrainLayer::AboveGround,
                hardness: None,
                transform_on_destroy: None,
                description: "平整坚实的常规地面，无移动阻碍与特殊物理效果。".to_string(),
            },
            Self::Wall => TerrainProperties {
                terrain: Self::Wall,
                name: "墙体".to_string(),
                symbol: "▤".to_string(),
                layer: TerrainLayer::AboveGround,
                hardness: Some(2),
                transform_on_destroy: Some(Self::Empty),
                description: "坚硬砖石掩体，阻挡角色移动与普通弹道。普通子弹无法击穿；高温穿甲弹可直接击碎并贯穿继续射击。撞击时有极高概率致死。".to_string(),
            },
            Self::Crate => TerrainProperties {
                terrain: Self::Crate,
                name: "木箱".to_string(),
                symbol: "▦".to_string(),
                layer: TerrainLayer::AboveGround,
                hardness: Some(1),
                transform_on_destroy: Some(Self::Empty),
                description: "轻质木制掩体，阻挡角色移动。普通子弹可击碎破坏后停下；穿甲弹击碎后可继续贯穿前行。".to_string(),
            },
            Self::Water => TerrainProperties {
                terrain: Self::Water,
                name: "水域".to_string(),
                symbol: "≈".to_string(),
                layer: TerrainLayer::AboveAndBelow,
                hardness: None,
                transform_on_destroy: None,
                description: "低洼深水区域，移动涉入时会遭受深水阻滞。若连续停留两回合将溺水淘汰。弹道直接掠过不受阻挡。暴风雪天气下相变为冰面。".to_string(),
            },
            Self::HighGround => TerrainProperties {
                terrain: Self::HighGround,
                name: "高地".to_string(),
                symbol: "△".to_string(),
                layer: TerrainLayer::Special,
                hardness: None,
                transform_on_destroy: None,
                description: "开阔的战术制高点，居高临下视野极佳。占据高地射击时有效射程额外增加 1 格。弹道可掠过高地。".to_string(),
            },
            Self::Mine => TerrainProperties {
                terrain: Self::Mine,
                name: "地雷".to_string(),
                symbol: "◆".to_string(),
                layer: TerrainLayer::Underground,
                hardness: None,
                transform_on_destroy: None,
                description: "埋伏于地表之下的烈性暗雷。子弹从上方空域飞过无法引爆或破坏；角色踏入进入生命周期检测，触发 85% 引爆或 10% 哑雷。".to_string(),
            },
            Self::Medkit => TerrainProperties {
                terrain: Self::Medkit,
                name: "护盾".to_string(),
                symbol: "✚".to_string(),
                layer: TerrainLayer::Special,
                hardness: None,
                transform_on_destroy: None,
                description: "散落的单兵便携充能护盾补给。角色踏入时拾取激活护盾，可完全抵消一次致命伤害；拾取后变为空地。".to_string(),
            },
            Self::Ice => TerrainProperties {
                terrain: Self::Ice,
                name: "冰面".to_string(),
                symbol: "❄".to_string(),
                layer: TerrainLayer::AboveGround,
                hardness: None,
                transform_on_destroy: None,
                description: "极度光滑的低温冰面，踏入极易触发滑行冲刺或失控打滑。离开冰面蹬地施力有 30% 概率震碎薄冰使其相变为深水。热浪下融化为水。".to_string(),
            },
        }
    }

    /// Returns the complete list of properties for all known terrains.
    #[must_use]
    pub fn all_properties() -> Vec<TerrainProperties> {
        vec![
            Self::Wall.properties(),
            Self::Crate.properties(),
            Self::Water.properties(),
            Self::Ice.properties(),
            Self::HighGround.properties(),
            Self::Mine.properties(),
            Self::Medkit.properties(),
            Self::Empty.properties(),
        ]
    }
}

/// Whether a player is controlled by a person or the local random bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKind {
    /// Human player using the browser.
    Human,
    /// In-process random bot.
    Bot,
}

/// Player life-cycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlayerStatus {
    /// Player may still take turns.
    Alive,
    /// Player has been removed from the map and turn order.
    Eliminated,
}

/// Player information stored in the authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlayerSetup {
    /// Display name.
    pub name: String,
    /// Human or bot controller.
    pub kind: PlayerKind,
}

/// Explicit inputs required to create a deterministic game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateGameConfig {
    /// Seed from which independent random streams are derived.
    pub seed: u64,
    /// Ordered players; the first player takes the first turn.
    pub players: Vec<PlayerSetup>,
}

/// Commands accepted by the MVP rules engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Collided violently with a wall or obstacle.
    Collision,
}

/// Structured facts emitted by the rules engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
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
    /// A weather or environmental shift occurred.
    WeatherChanged {
        /// Previous weather.
        from: Weather,
        /// New weather.
        to: Weather,
    },
    /// A structured dramatic event resolved by the event system.
    DramaticEvent {
        /// Indexed event identifier.
        event_id: EventId,
        /// Dramatic rarity tier.
        tier: EventTier,
        /// Headline title.
        title: String,
        /// Narrative summary text.
        narrative: String,
        /// Responsible or affected actor, if any.
        actor_id: Option<PlayerId>,
        /// Target position affected, if any.
        position: Option<Position>,
        /// BFS wave in the event chain (0 is root direct reaction).
        wave: u32,
        /// Monotonic sequence of the parent record that provoked this event.
        parent_sequence: Option<u64>,
    },
}

/// Machine-readable system notifications shown in the comprehensive log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameNotification {
    /// A new authoritative match was created.
    MatchStarted {
        /// Seed used for this match.
        seed: u64,
    },
    /// The match reached a terminal state.
    MatchFinished {
        /// Last surviving player, or none for a draw.
        winner_id: Option<PlayerId>,
    },
}

/// Severity used to render and route system notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    /// Ordinary informational notification.
    Info,
    /// Positive terminal or milestone notification.
    Success,
    /// Warning that deserves attention but does not reject a command.
    Warning,
}

/// One category of entry in the comprehensive chronological game log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum GameRecordContent {
    /// A player submitted an accepted command.
    Action {
        /// Acting player.
        actor_id: PlayerId,
        /// Accepted semantic command.
        command: PlayerCommand,
    },
    /// A fact occurred in the game world.
    Event {
        /// Structured event fact independent of any display text.
        event: GameEvent,
    },
    /// The turn cursor advanced.
    Turn {
        /// Player now expected to act.
        player_id: PlayerId,
        /// One-based round number.
        round: u32,
    },
    /// The host or rules system emitted a user-facing notification.
    Notification {
        /// Display and routing severity.
        level: NotificationLevel,
        /// Machine-readable notification payload.
        notification: GameNotification,
    },
}

/// Ordered entry in the comprehensive game log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GameRecord {
    /// Match-local monotonic sequence number, starting at one.
    pub sequence: u64,
    /// Typed record content. Categories are intentionally not parent/child
    /// links, so future ambient events and notifications can stand alone.
    #[serde(flatten)]
    pub content: GameRecordContent,
}

/// Terminal or running status of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Event system random stream.
    pub events: u64,
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
    /// Active ambient weather.
    pub weather: Weather,
    /// Sequence number assigned to the next comprehensive log entry.
    pub next_record_sequence: u64,
    /// Complete chronological record of actions, events, turns and notices.
    pub records: Vec<GameRecord>,
    /// Recent diagnostic traces of event pipeline tree resolutions.
    pub event_traces: Vec<PipelineTrace>,
    /// Consecutive rounds without any player elimination (stalemate escalator).
    pub rounds_without_elimination: u32,
}

/// Public map cell used by the Web client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CellView {
    /// Cell position.
    pub position: Position,
    /// Visible terrain.
    pub terrain: Terrain,
    /// Living player occupying the cell, if any.
    pub player_id: Option<PlayerId>,
}

/// Public snapshot returned to the local Web client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Active ambient weather.
    pub weather: Weather,
    /// Recent chronological actions, events, turns and notifications.
    pub records: Vec<GameRecord>,
    /// Recent diagnostic traces of event pipeline tree resolutions.
    pub event_traces: Vec<PipelineTrace>,
    /// Consecutive rounds without any player elimination (stalemate escalator).
    pub rounds_without_elimination: u32,
}

/// Lifecycle phase of a local lobby room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoomPhase {
    /// Humans and bots may join; the owner may configure and start the match.
    Waiting,
    /// A match is currently running.
    Playing,
    /// The current match finished; members may inspect its final state.
    Finished,
}

/// One human or bot displayed in the room roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoomMemberView {
    /// Stable identifier used for bot removal and owner transfer.
    pub id: RoomMemberId,
    /// Display name.
    pub name: String,
    /// Human tab or automated player.
    pub kind: PlayerKind,
    /// Whether this member currently owns the room.
    pub is_owner: bool,
    /// Whether this human member belongs to the requesting tab.
    pub is_self: bool,
    /// Match-local player identifier after the match starts.
    pub player_id: Option<PlayerId>,
}

/// Room card shown in the lobby.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoomSummary {
    /// Five-character room identifier.
    pub id: RoomId,
    /// Human-readable room name.
    pub name: String,
    /// Waiting, playing, or finished.
    pub phase: RoomPhase,
    /// Number of human tabs currently present.
    pub human_count: usize,
    /// Number of bot slots currently present.
    pub bot_count: usize,
    /// Maximum total member count supported by the MVP Core.
    pub capacity: usize,
    /// Whether joining requires the configured room password.
    pub password_required: bool,
    /// Whether the requesting tab is already a room member.
    pub is_member: bool,
}

/// Tab-specific lobby response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LobbyView {
    /// All active rooms.
    pub rooms: Vec<RoomSummary>,
    /// Room currently containing this tab, if any.
    pub current_room_id: Option<RoomId>,
}

/// Tab-specific room response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoomView {
    /// Five-character room identifier.
    pub id: RoomId,
    /// Human-readable room name.
    pub name: String,
    /// Waiting, playing, or finished.
    pub phase: RoomPhase,
    /// Password shown to all members after a successful join.
    pub password: Option<String>,
    /// Members in stable seat order, including bots.
    pub members: Vec<RoomMemberView>,
    /// Whether the requesting tab owns this room.
    pub is_owner: bool,
    /// Current match view after the room has started.
    pub game: Option<GameView>,
}

/// Caller role in the Russian Roulette system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum CallerRole {
    /// Neutral arbiter / moderator with privileged match controls and omniscient perspective.
    Referee,
    /// Individual match participant seat.
    Player {
        /// Player ID associated with this caller.
        player_id: PlayerId,
    },
}

/// Action to perform when referee configures bot slots in a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BotSetupAction {
    /// Add one bot slot to the waiting room.
    Add,
    /// Remove one bot slot from the waiting room.
    Remove,
}

/// Referee's unobstructed view of a single room member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefereeMemberView {
    /// Member identifier.
    pub id: RoomMemberId,
    /// Member display name.
    pub name: String,
    /// Human tab or automated bot.
    pub kind: PlayerKind,
    /// Whether this member holds room ownership.
    pub is_owner: bool,
    /// Assigned player ID in running/finished match, if any.
    pub player_id: Option<PlayerId>,
    /// Whether this human tab is actively connected and heartbeat-fresh.
    pub is_connected: bool,
}

/// Referee's omniscient, impartial view of an active or finished match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefereeGameView {
    /// Match seed.
    pub seed: u64,
    /// Authoritative state revision.
    pub revision: u64,
    /// Board edge length.
    pub map_size: u8,
    /// Full row-major cells with all occupants and terrain.
    pub cells: Vec<CellView>,
    /// Status and position of all players.
    pub players: Vec<PlayerState>,
    /// Player currently expected to act, if match is running.
    pub current_player_id: Option<PlayerId>,
    /// One-based round number.
    pub round: u32,
    /// Running or terminal status with winner ID.
    pub status: GameStatus,
    /// Active weather condition.
    pub weather: Weather,
    /// Count of currently alive players.
    pub alive_player_count: usize,
    /// Chronological history of actions, events, turns, and notifications.
    pub records: Vec<GameRecord>,
    /// Complete diagnostic traces of event pipeline tree resolutions.
    pub event_traces: Vec<PipelineTrace>,
    /// Consecutive rounds without player elimination.
    pub rounds_without_elimination: u32,
}

/// Omniscient room snapshot for the referee / host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefereeRoomView {
    /// Five-character room code.
    pub id: RoomId,
    /// Room display name.
    pub name: String,
    /// Current lifecycle phase.
    pub phase: RoomPhase,
    /// Cleartext password configured for this room, if any.
    pub password: Option<String>,
    /// Whether this room was created and is moderated by the referee.
    pub referee_managed: bool,
    /// All room members in stable seat order.
    pub members: Vec<RefereeMemberView>,
    /// Full match view if match has started.
    pub game: Option<RefereeGameView>,
}

/// Outcome of a referee-driven action or bot step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefereeStepResult {
    /// Room identifier.
    pub room_id: RoomId,
    /// Player who acted in this step, if any.
    pub acting_player_id: Option<PlayerId>,
    /// Semantic summary of the action executed.
    pub action_desc: String,
    /// State revision prior to execution.
    pub previous_revision: u64,
    /// New state revision after execution.
    pub new_revision: u64,
    /// Whether the match reached a terminal state on this step.
    pub is_match_finished: bool,
    /// Winner of the match if finished, or None.
    pub winner_player_id: Option<PlayerId>,
    /// Fresh room state snapshot after step.
    pub room: RefereeRoomView,
}

/// High-level room summary for the referee lobby listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefereeRoomSummary {
    /// Five-character room code.
    pub id: RoomId,
    /// Room display name.
    pub name: String,
    /// Current lifecycle phase.
    pub phase: RoomPhase,
    /// Count of human members.
    pub human_count: usize,
    /// Count of bot members.
    pub bot_count: usize,
    /// Room maximum capacity.
    pub capacity: usize,
    /// Whether password is required for human join.
    pub password_required: bool,
    /// Whether room is referee-managed.
    pub referee_managed: bool,
    /// Current match revision if match has started.
    pub current_revision: Option<u64>,
}

/// Result of dissolving a room via referee authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefereeDissolveResult {
    /// Successfully dissolved room ID.
    pub room_id: RoomId,
    /// Human-readable confirmation.
    pub message: String,
}
