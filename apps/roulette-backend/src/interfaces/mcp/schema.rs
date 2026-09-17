//! MCP tool schema definitions and metadata declarations.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

/// Returns the standard catalog of Russian Roulette MCP tool specifications.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn standard_tool_definitions() -> Vec<McpToolDefinition> {
    vec![
        McpToolDefinition {
            name: "list_lobby_rooms".to_string(),
            description: "查询俄罗斯轮盘游戏大厅的房间列表与活跃摘要".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "调用方客户端 TabId 标识"
                    }
                },
                "required": ["tab_id"]
            }),
        },
        McpToolDefinition {
            name: "get_room_view".to_string(),
            description: "查询指定房间的完整成员席位、阶段及对局棋盘快照".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "调用方客户端 TabId 标识"
                    },
                    "room_id": {
                        "type": "string",
                        "description": "五位房间号（大小写无关）"
                    }
                },
                "required": ["tab_id", "room_id"]
            }),
        },
        McpToolDefinition {
            name: "create_game_room".to_string(),
            description: "创建新的游戏房间，调用方自动成为房主".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "调用方客户端 TabId 标识"
                    },
                    "room_name": {
                        "type": "string",
                        "description": "房间展示名称"
                    },
                    "player_name": {
                        "type": "string",
                        "description": "房主玩家昵称"
                    },
                    "password": {
                        "type": "string",
                        "description": "可选加入密码"
                    }
                },
                "required": ["tab_id", "room_name", "player_name"]
            }),
        },
        McpToolDefinition {
            name: "join_game_room".to_string(),
            description: "加入指定的等待中房间".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "调用方客户端 TabId 标识"
                    },
                    "room_id": {
                        "type": "string",
                        "description": "五位房间号（大小写无关）"
                    },
                    "player_name": {
                        "type": "string",
                        "description": "玩家昵称"
                    },
                    "password": {
                        "type": "string",
                        "description": "房间密码（若有）"
                    }
                },
                "required": ["tab_id", "room_id", "player_name"]
            }),
        },
        McpToolDefinition {
            name: "start_game_room".to_string(),
            description: "房主在等待阶段启动对局，初始化确定性棋盘".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "房主 TabId 标识"
                    },
                    "room_id": {
                        "type": "string",
                        "description": "五位房间号"
                    },
                    "seed": {
                        "type": "integer",
                        "description": "可选确定性随机种子"
                    }
                },
                "required": ["tab_id", "room_id"]
            }),
        },
        McpToolDefinition {
            name: "submit_player_command".to_string(),
            description: "在对局中提交当前玩家行动（移动、射击、等待、自杀）".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "当前行动玩家的 TabId 标识"
                    },
                    "room_id": {
                        "type": "string",
                        "description": "五位房间号"
                    },
                    "expected_revision": {
                        "type": "integer",
                        "description": "当前对局状态版本号，用于乐观并发校验"
                    },
                    "command": {
                        "type": "object",
                        "description": "玩家指令，例如 {\"action\": \"shoot\", \"direction\": \"up\"}"
                    }
                },
                "required": ["tab_id", "room_id", "expected_revision", "command"]
            }),
        },
        McpToolDefinition {
            name: "step_bot_action".to_string(),
            description: "房主单步推进轮到行动的 Bot，执行一次决策".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "房主 TabId 标识"
                    },
                    "room_id": {
                        "type": "string",
                        "description": "五位房间号"
                    },
                    "expected_revision": {
                        "type": "integer",
                        "description": "当前对局状态版本号"
                    }
                },
                "required": ["tab_id", "room_id", "expected_revision"]
            }),
        },
        McpToolDefinition {
            name: "get_terrain_properties".to_string(),
            description: "查询所有地形层级、硬度阻挡与穿甲弹破坏规则".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}
