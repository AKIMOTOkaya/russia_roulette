//! MCP server implementation for Russian Roulette referee and moderator role.

#![forbid(unsafe_code)]

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_handler, tool_router};
use roulette_host::{GameService, ServiceError};
use serde_json::json;

use crate::interfaces::mcp::schema::{
    RefereeCreateRoomParams, RefereeDissolveRoomParams, RefereeForceCommandParams,
    RefereeInspectRoomParams, RefereeListRoomsParams, RefereeQueryRulesParams,
    RefereeRenderRoomImageParams, RefereeSetupBotsParams, RefereeStartMatchParams,
    RefereeStepBotParams,
};

/// Russian Roulette Referee and Moderator MCP server.
#[derive(Clone)]
pub struct RussianRouletteMcpServer {
    service: Arc<GameService>,
}

#[tool_router]
impl RussianRouletteMcpServer {
    /// Creates a new MCP server instance wrapping `GameService`.
    #[must_use]
    pub fn new(service: Arc<GameService>) -> Self {
        Self { service }
    }

    /// List all rooms in the game lobby with their current phase, members, and revision.
    #[tool(description = "裁判列出大厅内所有房间的概览摘要与当前状态，支持按阶段筛选")]
    async fn referee_list_rooms(
        &self,
        Parameters(params): Parameters<RefereeListRoomsParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_list_rooms() {
            Ok(mut rooms) => {
                if let Some(phase) = params.filter_phase {
                    rooms.retain(|r| r.phase == phase);
                }
                Ok(format_success(&rooms))
            }
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Inspect any game room with unobstructed omniscient referee visibility.
    #[tool(
        description = "裁判以全知无遮挡视角检查指定房间详情，包括地图网格、玩家坐标血量及完整事件执行树"
    )]
    async fn referee_inspect_room(
        &self,
        Parameters(params): Parameters<RefereeInspectRoomParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_inspect_room(&params.room_id) {
            Ok(view) => Ok(format_success(&view)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Create a new game room moderated by the referee, optionally pre-populating with bots.
    #[tool(description = "裁判主持创建新的比赛房间，可设置房间名、密码并预置 Bot 席位")]
    async fn referee_create_room(
        &self,
        Parameters(params): Parameters<RefereeCreateRoomParams>,
    ) -> Result<CallToolResult, McpError> {
        let initial_bots = params.initial_bots.unwrap_or(0);
        match self.service.referee_create_room(
            &params.room_name,
            params.password.as_deref(),
            initial_bots,
        ) {
            Ok(view) => Ok(format_success(&view)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Add or remove bot slots in a waiting room.
    #[tool(description = "裁判在等待阶段为指定房间增添或移除 Bot 席位")]
    async fn referee_setup_bots(
        &self,
        Parameters(params): Parameters<RefereeSetupBotsParams>,
    ) -> Result<CallToolResult, McpError> {
        let bot_member_id = params.bot_member_id.map(roulette_domain::RoomMemberId);
        match self.service.referee_setup_bots(
            &params.room_id,
            params.action,
            bot_member_id.as_ref(),
            params.expected_revision,
        ) {
            Ok(view) => Ok(format_success(&view)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Start a match in the waiting room with optional deterministic random seed.
    #[tool(description = "裁判正式开启比赛对局，支持指定确定性随机种子以便复现与审计")]
    async fn referee_start_match(
        &self,
        Parameters(params): Parameters<RefereeStartMatchParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_start_match(
            &params.room_id,
            params.seed,
            params.expected_revision,
            params.idempotency_key.as_deref(),
        ) {
            Ok(view) => Ok(format_success(&view)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Advance the active bot by one step.
    #[tool(description = "裁判推进当前轮到的 Bot 执行单步决策，返回动作描述与新的状态版本号")]
    async fn referee_step_bot(
        &self,
        Parameters(params): Parameters<RefereeStepBotParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_step_bot(
            &params.room_id,
            params.expected_revision,
            params.idempotency_key.as_deref(),
        ) {
            Ok(result) => Ok(format_success(&result)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Arbitrate or force the current player turn to execute an action.
    #[tool(description = "裁判强制代行/强裁当前轮次玩家执行指定指令（移动、射击、等待、自杀）")]
    async fn referee_force_command(
        &self,
        Parameters(params): Parameters<RefereeForceCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_force_command(
            &params.room_id,
            params.expected_revision,
            params.command,
            params.idempotency_key.as_deref(),
        ) {
            Ok(result) => Ok(format_success(&result)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Permanently close and dissolve a room.
    #[tool(description = "裁判强制关闭并解散指定房间")]
    async fn referee_dissolve_room(
        &self,
        Parameters(params): Parameters<RefereeDissolveRoomParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_dissolve_room(&params.room_id) {
            Ok(result) => Ok(format_success(&result)),
            Err(err) => Ok(format_service_error(&err)),
        }
    }

    /// Query canonical terrain rules and penetration mechanics.
    #[tool(description = "查询游戏权威规则文档、特殊地形层级与穿甲弹破坏机制")]
    async fn referee_query_rules(
        &self,
        Parameters(_params): Parameters<RefereeQueryRulesParams>,
    ) -> Result<CallToolResult, McpError> {
        let rules = self.service.get_terrain_rules();
        Ok(format_success(&rules))
    }

    /// Render match state and tactical board into an image (PNG base64 or SVG).
    #[tool(
        description = "裁判将当前房间与战术棋盘按规则渲染合成精美HUD卡片图片（PNG base64或SVG格式），便于在QQ等IM会话中直观输出战况"
    )]
    async fn referee_render_room_image(
        &self,
        Parameters(params): Parameters<RefereeRenderRoomImageParams>,
    ) -> Result<CallToolResult, McpError> {
        match self.service.referee_inspect_room(&params.room_id) {
            Ok(room) => {
                if params.format.as_deref() == Some("svg") {
                    let svg = roulette_renderer::render_match_svg(&room);
                    Ok(format_success(&json!({
                        "room_id": params.room_id,
                        "format": "svg",
                        "content_type": "image/svg+xml",
                        "svg": svg,
                    })))
                } else {
                    match roulette_renderer::render_match_png(&room) {
                        Ok(png_bytes) => {
                            let b64 = crate::interfaces::mcp::dispatcher::base64_encode(&png_bytes);
                            Ok(format_success(&json!({
                                "room_id": params.room_id,
                                "format": "png",
                                "content_type": "image/png",
                                "size_bytes": png_bytes.len(),
                                "base64": b64,
                                "data_uri": format!("data:image/png;base64,{b64}"),
                            })))
                        }
                        Err(err) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                            "Render error: {err}"
                        ))])),
                    }
                }
            }
            Err(err) => Ok(format_service_error(&err)),
        }
    }
}

#[tool_handler]
impl ServerHandler for RussianRouletteMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("russian-roulette-referee", "0.1.0"))
            .with_instructions("Russian Roulette Referee & Moderator MCP Server. Provides omniscient match inspection, bot step execution, deterministic rules arbitrating, and room orchestration.")
    }
}

fn format_success<T: serde::Serialize>(value: &T) -> CallToolResult {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("Serialization error: {e}"));
    CallToolResult::success(vec![ContentBlock::text(text)])
}

fn format_service_error(err: &ServiceError) -> CallToolResult {
    let (code, msg) = match err {
        ServiceError::BadRequest(msg) => ("BAD_REQUEST", msg.clone()),
        ServiceError::NotFound(msg) => ("NOT_FOUND", msg.clone()),
        ServiceError::Forbidden(msg) => ("FORBIDDEN", msg.clone()),
        ServiceError::Conflict(msg) => {
            if msg.starts_with("revision conflict") {
                ("REVISION_CONFLICT", msg.clone())
            } else {
                ("CONFLICT", msg.clone())
            }
        }
        ServiceError::Internal(msg) => ("INTERNAL_ERROR", msg.clone()),
    };

    let error_payload = json!({
        "error_code": code,
        "message": msg,
    });
    let text = serde_json::to_string_pretty(&error_payload)
        .unwrap_or_else(|_| format!("{{\"error_code\": \"{code}\", \"message\": \"{msg}\"}}"));
    CallToolResult::error(vec![ContentBlock::text(text)])
}
