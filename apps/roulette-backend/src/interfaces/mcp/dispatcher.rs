//! MCP tool dispatcher delegating tool invocations directly to `GameService`.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use roulette_domain::{BotSetupAction, PlayerCommand, RoomMemberId, RoomPhase, TabId};
use roulette_host::{CreateRoomConfig, GameService, JoinRoomConfig};
use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};

use crate::interfaces::mcp::schema::{McpToolDefinition, standard_tool_definitions};

/// Error produced during MCP tool parameter parsing or execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// JSON-RPC compatible error code.
    pub code: i32,
    /// Human-readable error description.
    pub message: String,
}

impl Display for McpError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "MCP Error {}: {}", self.code, self.message)
    }
}

impl Error for McpError {}

impl McpError {
    /// Constructs an invalid parameters error.
    #[must_use]
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: msg.into(),
        }
    }

    /// Constructs a tool execution failure error.
    #[must_use]
    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self {
            code: -32000,
            message: msg.into(),
        }
    }

    /// Constructs a tool not found error.
    #[must_use]
    pub fn tool_not_found(name: &str) -> Self {
        Self {
            code: -32601,
            message: format!("tool '{name}' not found"),
        }
    }
}

/// Dispatches MCP tool calls to the application `GameService`.
#[derive(Debug, Clone)]
pub struct McpDispatcher {
    service: Arc<GameService>,
    tools: Vec<McpToolDefinition>,
}

impl McpDispatcher {
    /// Creates a new MCP tool dispatcher.
    #[must_use]
    pub fn new(service: Arc<GameService>) -> Self {
        Self {
            service,
            tools: standard_tool_definitions(),
        }
    }

    /// Returns the registered list of MCP tool definitions.
    #[must_use]
    pub fn tools(&self) -> &[McpToolDefinition] {
        &self.tools
    }

    /// Dispatches an MCP tool call by name with JSON arguments to `GameService`.
    ///
    /// # Errors
    ///
    /// Returns `McpError` if tool is unknown, parameters are invalid, or execution fails.
    #[allow(clippy::too_many_lines)]
    pub fn dispatch(&self, tool_name: &str, arguments: &Value) -> Result<Value, McpError> {
        match tool_name {
            "referee_list_rooms" => {
                let phase: Option<RoomPhase> = arguments
                    .get("filter_phase")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                let mut rooms = self
                    .service
                    .referee_list_rooms()
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                if let Some(p) = phase {
                    rooms.retain(|r| r.phase == p);
                }
                to_value(rooms).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_inspect_room" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let view = self
                    .service
                    .referee_inspect_room(&room_id)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_create_room" => {
                let room_name: String = parse_arg(arguments, "room_name")?;
                let password: Option<String> = arguments
                    .get("password")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let initial_bots: usize = arguments
                    .get("initial_bots")
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .unwrap_or(0);

                let view = self
                    .service
                    .referee_create_room(&room_name, password.as_deref(), initial_bots)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_setup_bots" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let action: BotSetupAction = parse_arg(arguments, "action")?;
                let bot_member_id: Option<String> = arguments
                    .get("bot_member_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let expected_revision: u64 = parse_arg(arguments, "expected_revision")?;
                let member_id = bot_member_id.map(RoomMemberId);

                let view = self
                    .service
                    .referee_setup_bots(&room_id, action, member_id.as_ref(), expected_revision)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_start_match" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let seed: Option<u64> = arguments.get("seed").and_then(Value::as_u64);
                let expected_revision: u64 = parse_arg(arguments, "expected_revision")?;
                let idempotency_key: Option<&str> =
                    arguments.get("idempotency_key").and_then(Value::as_str);

                let view = self
                    .service
                    .referee_start_match(&room_id, seed, expected_revision, idempotency_key)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_step_bot" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let expected_revision: u64 = parse_arg(arguments, "expected_revision")?;
                let idempotency_key: Option<&str> =
                    arguments.get("idempotency_key").and_then(Value::as_str);

                let result = self
                    .service
                    .referee_step_bot(&room_id, expected_revision, idempotency_key)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(result).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_force_command" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let expected_revision: u64 = parse_arg(arguments, "expected_revision")?;
                let command: PlayerCommand = parse_arg(arguments, "command")?;
                let idempotency_key: Option<&str> =
                    arguments.get("idempotency_key").and_then(Value::as_str);

                let result = self
                    .service
                    .referee_force_command(&room_id, expected_revision, command, idempotency_key)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(result).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_dissolve_room" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let result = self
                    .service
                    .referee_dissolve_room(&room_id)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(result).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_query_rules" | "get_terrain_properties" => {
                let rules = self.service.get_terrain_rules();
                to_value(rules).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "referee_render_room_image" => {
                let room_id: String = parse_arg(arguments, "room_id")?;
                let format_str: Option<String> = arguments
                    .get("format")
                    .and_then(Value::as_str)
                    .map(str::to_lowercase);
                let room = self
                    .service
                    .referee_inspect_room(&room_id)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;

                if format_str.as_deref() == Some("svg") {
                    let svg = roulette_renderer::render_match_svg(&room);
                    Ok(serde_json::json!({
                        "room_id": room_id,
                        "format": "svg",
                        "content_type": "image/svg+xml",
                        "svg": svg,
                    }))
                } else {
                    let png_bytes = roulette_renderer::render_match_png(&room)
                        .map_err(|e| McpError::execution_failed(e.to_string()))?;
                    let base64_encoded = base64_encode(&png_bytes);
                    let data_uri = format!("data:image/png;base64,{base64_encoded}");
                    Ok(serde_json::json!({
                        "room_id": room_id,
                        "format": "png",
                        "content_type": "image/png",
                        "size_bytes": png_bytes.len(),
                        "base64": base64_encoded,
                        "data_uri": data_uri,
                    }))
                }
            }
            "list_lobby_rooms" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let view = self
                    .service
                    .get_lobby_view(&TabId(tab_id))
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "get_room_view" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let room_id: String = parse_arg(arguments, "room_id")?;
                let view = self
                    .service
                    .get_room_view(&room_id, &TabId(tab_id))
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "create_game_room" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let room_name: String = parse_arg(arguments, "room_name")?;
                let player_name: String = parse_arg(arguments, "player_name")?;
                let password: Option<String> = arguments
                    .get("password")
                    .and_then(Value::as_str)
                    .map(str::to_owned);

                let view = self
                    .service
                    .create_room(&CreateRoomConfig {
                        tab_id: TabId(tab_id),
                        room_name,
                        player_name,
                        password,
                    })
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "join_game_room" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let room_id: String = parse_arg(arguments, "room_id")?;
                let player_name: String = parse_arg(arguments, "player_name")?;
                let password: Option<String> = arguments
                    .get("password")
                    .and_then(Value::as_str)
                    .map(str::to_owned);

                let view = self
                    .service
                    .join_room(&JoinRoomConfig {
                        tab_id: TabId(tab_id),
                        room_id,
                        player_name,
                        password,
                    })
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "start_game_room" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let room_id: String = parse_arg(arguments, "room_id")?;
                let seed: Option<u64> = arguments.get("seed").and_then(Value::as_u64);

                let view = self
                    .service
                    .start_room(&room_id, &TabId(tab_id), seed)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "submit_player_command" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let room_id: String = parse_arg(arguments, "room_id")?;
                let expected_revision: u64 = parse_arg(arguments, "expected_revision")?;
                let cmd_value = arguments
                    .get("command")
                    .ok_or_else(|| McpError::invalid_params("missing 'command' parameter"))?;
                let command: PlayerCommand =
                    serde_json::from_value(cmd_value.clone()).map_err(|e| {
                        McpError::invalid_params(format!("invalid command payload: {e}"))
                    })?;

                let view = self
                    .service
                    .submit_command(&room_id, &TabId(tab_id), expected_revision, command)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            "step_bot_action" => {
                let tab_id: String = parse_arg(arguments, "tab_id")?;
                let room_id: String = parse_arg(arguments, "room_id")?;
                let expected_revision: u64 = parse_arg(arguments, "expected_revision")?;

                let view = self
                    .service
                    .step_bot(&room_id, &TabId(tab_id), expected_revision)
                    .map_err(|e| McpError::execution_failed(e.to_string()))?;
                to_value(view).map_err(|e| McpError::execution_failed(e.to_string()))
            }
            unknown => Err(McpError::tool_not_found(unknown)),
        }
    }
}

fn parse_arg<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, McpError> {
    args.get(name)
        .cloned()
        .ok_or_else(|| McpError::invalid_params(format!("missing required parameter '{name}'")))
        .and_then(|val| {
            serde_json::from_value(val)
                .map_err(|e| McpError::invalid_params(format!("invalid parameter '{name}': {e}")))
        })
}

/// Encodes raw bytes into standard Base64 string without external dependencies.
#[must_use]
pub fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        result.push(CHARSET[(b0 >> 2) as usize] as char);
        result.push(CHARSET[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
