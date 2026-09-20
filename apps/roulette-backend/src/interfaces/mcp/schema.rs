//! MCP tool schema definitions, parameter contracts, and metadata declarations.

#![forbid(unsafe_code)]

use roulette_domain::{BotSetupAction, PlayerCommand, RoomPhase};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};

/// An MCP Tool definition describing name, human description, and JSON schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Unique identifier of the tool.
    pub name: String,
    /// Human description of what this tool accomplishes.
    pub description: String,
    /// JSON Schema for input arguments.
    pub input_schema: Value,
}

/// Parameters for `referee_list_rooms`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RefereeListRoomsParams {
    /// Optional filter by room lifecycle phase.
    pub filter_phase: Option<RoomPhase>,
}

/// Parameters for `referee_inspect_room`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeInspectRoomParams {
    /// Five-character case-insensitive room short code.
    pub room_id: String,
}

/// Parameters for `referee_create_room`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeCreateRoomParams {
    /// Human-readable room display name.
    pub room_name: String,
    /// Optional room password.
    pub password: Option<String>,
    /// Number of initial bot seats to pre-populate (0..=6).
    pub initial_bots: Option<usize>,
    /// Optional client idempotency key to protect against retry duplicates.
    pub idempotency_key: Option<String>,
}

/// Parameters for `referee_setup_bots`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeSetupBotsParams {
    /// Five-character room identifier.
    pub room_id: String,
    /// Action to perform: add or remove bot.
    pub action: BotSetupAction,
    /// Specific bot member ID to remove (optional; defaults to the last bot).
    pub bot_member_id: Option<String>,
    /// Expected match or room revision for optimistic concurrency control.
    pub expected_revision: u64,
}

/// Parameters for `referee_start_match`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeStartMatchParams {
    /// Five-character room identifier.
    pub room_id: String,
    /// Optional deterministic random seed. If omitted, server generates one.
    pub seed: Option<u64>,
    /// Expected room revision (must be 0 for starting match).
    pub expected_revision: u64,
    /// Optional idempotency key.
    pub idempotency_key: Option<String>,
}

/// Parameters for `referee_step_bot`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeStepBotParams {
    /// Five-character room identifier.
    pub room_id: String,
    /// Authoritative state revision expected prior to step.
    pub expected_revision: u64,
    /// Optional idempotency key.
    pub idempotency_key: Option<String>,
}

/// Parameters for `referee_force_command`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeForceCommandParams {
    /// Five-character room identifier.
    pub room_id: String,
    /// Authoritative state revision expected prior to command.
    pub expected_revision: u64,
    /// Exact player command for the current turn owner to perform.
    pub command: PlayerCommand,
    /// Optional idempotency key.
    pub idempotency_key: Option<String>,
}

/// Parameters for `referee_dissolve_room`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeDissolveRoomParams {
    /// Five-character room identifier to dissolve.
    pub room_id: String,
}

/// Parameters for `referee_query_rules`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RefereeQueryRulesParams {}

/// Parameters for `referee_render_room_image`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefereeRenderRoomImageParams {
    /// Five-character room identifier.
    pub room_id: String,
    /// Output format: "png" (default base64 data URI) or "svg" (raw SVG XML).
    pub format: Option<String>,
    /// View mode: "full" (default complete HUD card) or "board" (standalone dynamic board).
    pub view_mode: Option<String>,
    /// Optional cell size in pixels for standalone board (default 96).
    pub cell_size: Option<u32>,
}

/// Parameters for `referee_render_asset_sheet`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RefereeRenderAssetSheetParams {
    /// Output format: "png" (default base64 data URI) or "svg" (raw SVG XML).
    pub format: Option<String>,
}

/// Parameters for `create_game_room`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateGameRoomParams {
    /// Tab/Agent identity code.
    pub tab_id: String,
    /// Room display name.
    pub room_name: String,
    /// Player display name.
    pub player_name: String,
    /// Optional room password.
    pub password: Option<String>,
}

/// Parameters for `list_lobby_rooms`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListLobbyRoomsParams {
    /// Tab/Agent identity code.
    pub tab_id: String,
}

/// Parameters for `get_room_view`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetRoomViewParams {
    /// Tab/Agent identity code.
    pub tab_id: String,
    /// Five-character room code.
    pub room_id: String,
}

/// Parameters for `join_game_room`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JoinGameRoomParams {
    /// Tab/Agent identity code.
    pub tab_id: String,
    /// Five-character room code.
    pub room_id: String,
    /// Player display name.
    pub player_name: String,
    /// Optional room password.
    pub password: Option<String>,
}

/// Parameters for `start_game_room`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartGameRoomParams {
    /// Tab/Agent identity code.
    pub tab_id: String,
    /// Five-character room code.
    pub room_id: String,
    /// Optional random seed.
    pub seed: Option<u64>,
}

/// Parameters for `submit_game_command`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubmitGameCommandParams {
    /// Tab/Agent identity code.
    pub tab_id: String,
    /// Five-character room code.
    pub room_id: String,
    /// Expected match revision for concurrency check.
    pub expected_revision: u64,
    /// Player command to submit.
    pub command: PlayerCommand,
}

/// Parameters for `get_terrain_properties`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct GetTerrainPropertiesParams {}

/// Parameters for `get_object_properties`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct GetObjectPropertiesParams {}

/// Returns the standard catalog of Russian Roulette Referee MCP tool specifications.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn standard_tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "referee_list_rooms".to_string(),
            description: "裁判列出大厅内所有房间的概览摘要与当前状态".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeListRoomsParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_inspect_room".to_string(),
            description: "裁判以全知无遮挡视角检查指定房间详情，包括地图、玩家血量及事件执行流"
                .to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeInspectRoomParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_create_room".to_string(),
            description: "裁判主持创建新的比赛房间，可指定房间名并预置 Bot 席位".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeCreateRoomParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_setup_bots".to_string(),
            description: "裁判在等待阶段为指定房间增添或移除 Bot 席位".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeSetupBotsParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_start_match".to_string(),
            description: "裁判正式开启比赛对局，支持指定确定性随机种子".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeStartMatchParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_step_bot".to_string(),
            description: "裁判推进当前轮到的 Bot 执行单步智能决策与状态演化".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeStepBotParams)).unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_force_command".to_string(),
            description: "裁判强制代行/强裁当前行动玩家执行指定指令（移动、射击、等待、自杀）"
                .to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeForceCommandParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_dissolve_room".to_string(),
            description: "裁判强制关闭并解散指定房间".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeDissolveRoomParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_query_rules".to_string(),
            description: "查询权威规则、特殊地形硬度属性与穿甲弹破坏机制".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeQueryRulesParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_render_room_image".to_string(),
            description: "裁判将当前房间与战术棋盘按规则渲染合成精美HUD卡片图片（PNG base64或SVG格式），便于在QQ等IM会话中直观输出战况".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeRenderRoomImageParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "referee_render_asset_sheet".to_string(),
            description: "渲染纯矢量战术素材图谱总览（地形、玩家席位、弹道激光特效、天气徽记）".to_string(),
            input_schema: to_value(schemars::schema_for!(RefereeRenderAssetSheetParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "create_game_room".to_string(),
            description: "作为玩家创建新的比赛房间".to_string(),
            input_schema: to_value(schemars::schema_for!(CreateGameRoomParams)).unwrap_or_default(),
        },
        McpToolDefinition {
            name: "list_lobby_rooms".to_string(),
            description: "作为玩家列出大厅中的可见房间".to_string(),
            input_schema: to_value(schemars::schema_for!(ListLobbyRoomsParams)).unwrap_or_default(),
        },
        McpToolDefinition {
            name: "get_room_view".to_string(),
            description: "作为玩家获取指定房间的当前视角".to_string(),
            input_schema: to_value(schemars::schema_for!(GetRoomViewParams)).unwrap_or_default(),
        },
        McpToolDefinition {
            name: "join_game_room".to_string(),
            description: "作为玩家加入指定房间".to_string(),
            input_schema: to_value(schemars::schema_for!(JoinGameRoomParams)).unwrap_or_default(),
        },
        McpToolDefinition {
            name: "start_game_room".to_string(),
            description: "房主开始当前房间的游戏对局".to_string(),
            input_schema: to_value(schemars::schema_for!(StartGameRoomParams)).unwrap_or_default(),
        },
        McpToolDefinition {
            name: "submit_game_command".to_string(),
            description: "玩家向当前对局提交行动指令".to_string(),
            input_schema: to_value(schemars::schema_for!(SubmitGameCommandParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "get_terrain_properties".to_string(),
            description: "获取地图基础地面地形（平地、水域、高地、冰面）物理与战术规则".to_string(),
            input_schema: to_value(schemars::schema_for!(GetTerrainPropertiesParams))
                .unwrap_or_default(),
        },
        McpToolDefinition {
            name: "get_object_properties".to_string(),
            description: "获取地图上覆物体（掩体、陷阱、补给道具）权威属性与破坏规则".to_string(),
            input_schema: to_value(schemars::schema_for!(GetObjectPropertiesParams))
                .unwrap_or_default(),
        },
    ]
}
