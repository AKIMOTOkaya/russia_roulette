//! Shared Russian Roulette game backend engine and HTTP / MCP interface layer.

#![forbid(unsafe_code)]

pub mod interfaces;

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

pub use interfaces::http::build_router;
use roulette_domain::ServerMode;
use roulette_host::{GameManager, GameService, InMemoryGameRepository, generate_seed};

/// Configuration options for the Russian Roulette server runtime.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Host IP address string to bind.
    pub host: String,
    /// TCP port number to listen on.
    pub port: u16,
    /// Server mode: Local LAN vs Public Central Server.
    pub server_mode: ServerMode,
    /// Founder temporary authentication password (used in local mode).
    pub founder_password: String,
    /// Whether LAN exposure is enabled by default.
    pub lan_exposed: bool,
    /// Optional administrator token (used in public server mode).
    pub admin_token: Option<String>,
}

impl ServerConfig {
    /// Returns standard default configuration for local LAN execution.
    #[must_use]
    pub fn local_defaults() -> Self {
        Self {
            host: configured_host(),
            port: configured_port(8787),
            server_mode: ServerMode::Local,
            founder_password: configured_founder_password(),
            lan_exposed: env::var("ROULETTE_LAN_EXPOSED")
                .is_ok_and(|v| v.eq_ignore_ascii_case("true") || v == "1"),
            admin_token: None,
        }
    }

    /// Returns standard default configuration for public central server execution.
    #[must_use]
    pub fn public_defaults() -> Self {
        Self {
            host: configured_host(),
            port: configured_port(8080),
            server_mode: ServerMode::Public,
            founder_password: String::new(),
            lan_exposed: true,
            admin_token: configured_admin_token(),
        }
    }

    /// Derives configuration dynamically from environment variables and CLI arguments.
    #[must_use]
    pub fn from_env() -> Self {
        let is_public = env::var("ROULETTE_SERVER_MODE")
            .is_ok_and(|v| v.eq_ignore_ascii_case("public"))
            || env::args().any(|arg| arg == "--public" || arg == "--mode=public");

        if is_public {
            Self::public_defaults()
        } else {
            Self::local_defaults()
        }
    }
}

/// Runs server with custom configuration until shutdown signal is received.
///
/// # Errors
///
/// Returns error if network socket cannot be bound.
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let address = SocketAddr::new(config.host.parse::<IpAddr>()?, config.port);

    // 1. Persistence Layer: Repository (InMemory default, prepared for SQLite / PostgreSQL)
    let repository = Arc::new(InMemoryGameRepository::new());

    // 2. Domain Management Layer: GameManager
    let manager = Arc::new(GameManager::new(repository, generate_seed()));

    // 3. Application Service Layer: GameService
    let service = match config.server_mode {
        ServerMode::Local => Arc::new(GameService::new(
            manager,
            config.founder_password.clone(),
            config.lan_exposed,
        )),
        ServerMode::Public => {
            let admin_token = config
                .admin_token
                .clone()
                .unwrap_or_else(|| config.founder_password.clone());
            Arc::new(GameService::new_public(manager, admin_token))
        }
    };

    // 4. Interface Layer: HTTP + MCP
    let app = build_router(Arc::clone(&service));

    let listener = tokio::net::TcpListener::bind(address).await?;
    match config.server_mode {
        ServerMode::Local => {
            println!(
                "俄罗斯轮盘【局域网/单机模式】：http://127.0.0.1:{}",
                config.port
            );
            println!(
                "MCP 工具端点：http://127.0.0.1:{}/api/mcp/tools",
                config.port
            );
            println!("MCP Streamable 端点：http://127.0.0.1:{}/mcp", config.port);
            println!("创始人临时密码：{}", config.founder_password);
            println!(
                "局域网访问：{}",
                if config.lan_exposed {
                    "已开启"
                } else {
                    "默认关闭（可凭创始人密码在 Web 设置中开启）"
                }
            );
        }
        ServerMode::Public => {
            println!(
                "俄罗斯轮盘【公网中央服务器模式】：http://0.0.0.0:{}",
                config.port
            );
            println!("MCP 工具端点：http://0.0.0.0:{}/api/mcp/tools", config.port);
            println!("MCP Streamable 端点：http://0.0.0.0:{}/mcp", config.port);
            if let Some(ref token) = config.admin_token {
                println!("管理员凭据：已配置 ({} 字符)", token.len());
            }
            println!("网络访问：公网反向代理已就绪，全网放行");
        }
    }

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

/// Runs server in local mode.
///
/// # Errors
///
/// Returns error on network bind failure.
pub async fn run_local() -> Result<(), Box<dyn std::error::Error>> {
    run_server(ServerConfig::local_defaults()).await
}

/// Runs server in public central mode.
///
/// # Errors
///
/// Returns error on network bind failure.
pub async fn run_public_server() -> Result<(), Box<dyn std::error::Error>> {
    run_server(ServerConfig::public_defaults()).await
}

/// Runs server from environment variables.
///
/// # Errors
///
/// Returns error on network bind failure.
pub async fn run_from_env() -> Result<(), Box<dyn std::error::Error>> {
    run_server(ServerConfig::from_env()).await
}

fn configured_founder_password() -> String {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if let Some(value) = argument.strip_prefix("--founder-password=")
            && !value.is_empty()
        {
            return value.to_owned();
        }
        if argument == "--founder-password"
            && let Some(value) = arguments.next().filter(|value| !value.is_empty())
        {
            return value;
        }
    }
    env::var("ROULETTE_FOUNDER_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("{:08X}", generate_seed() & u64::from(u32::MAX)))
}

fn configured_admin_token() -> Option<String> {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if let Some(value) = argument.strip_prefix("--admin-token=")
            && !value.is_empty()
        {
            return Some(value.to_owned());
        }
        if argument == "--admin-token"
            && let Some(value) = arguments.next().filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }
    env::var("ROULETTE_ADMIN_TOKEN")
        .ok()
        .filter(|value| !value.is_empty())
}

fn configured_port(default_port: u16) -> u16 {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if let Some(value) = argument.strip_prefix("--port=")
            && let Ok(p) = value.parse::<u16>()
        {
            return p;
        }
        if argument == "--port"
            && let Some(value) = arguments.next().and_then(|v| v.parse::<u16>().ok())
        {
            return value;
        }
    }
    env::var("ROULETTE_HTTP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default_port)
}

fn configured_host() -> String {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if let Some(value) = argument.strip_prefix("--host=")
            && !value.is_empty()
        {
            return value.to_owned();
        }
        if argument == "--host"
            && let Some(value) = arguments.next().filter(|value| !value.is_empty())
        {
            return value;
        }
    }
    env::var("ROULETTE_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn test_mcp_dispatcher_direct_flow() {
        let repo = Arc::new(InMemoryGameRepository::new());
        let manager = Arc::new(GameManager::new(repo, 42));
        let service = Arc::new(GameService::new(manager, "testpass".to_string(), false));
        let dispatcher = interfaces::mcp::McpDispatcher::new(service);

        // 1. Check tools schema catalog
        let tools = dispatcher.tools();
        assert!(tools.iter().any(|t| t.name == "create_game_room"));
        assert!(tools.iter().any(|t| t.name == "list_lobby_rooms"));
        assert!(tools.iter().any(|t| t.name == "get_terrain_properties"));

        // 2. Dispatch terrain rules tool
        let terrain_res = dispatcher
            .dispatch("get_terrain_properties", &json!({}))
            .expect("dispatch terrain rules");
        assert!(terrain_res.is_array());

        // 3. Dispatch create room tool
        let create_res = dispatcher
            .dispatch(
                "create_game_room",
                &json!({
                    "tab_id": "mcp-agent-1",
                    "room_name": "MCP战局",
                    "player_name": "Agent Alpha"
                }),
            )
            .expect("create room via mcp");

        let room_id = create_res
            .get("id")
            .and_then(|id| id.get("0"))
            .or_else(|| create_res.get("id"))
            .and_then(serde_json::Value::as_str)
            .expect("room id");

        // 4. Dispatch list lobby
        let lobby_res = dispatcher
            .dispatch(
                "list_lobby_rooms",
                &json!({
                    "tab_id": "mcp-agent-1"
                }),
            )
            .expect("list lobby via mcp");

        let rooms_array = lobby_res
            .get("rooms")
            .and_then(serde_json::Value::as_array)
            .expect("rooms array");
        assert_eq!(rooms_array.len(), 1);

        // 5. Dispatch join room
        let _join_res = dispatcher
            .dispatch(
                "join_game_room",
                &json!({
                    "tab_id": "mcp-agent-2",
                    "room_id": room_id,
                    "player_name": "Agent Beta"
                }),
            )
            .expect("join room via mcp");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn test_mcp_referee_full_lifecycle() {
        let repo = Arc::new(InMemoryGameRepository::new());
        let manager = Arc::new(GameManager::new(repo, 100));
        let service = Arc::new(GameService::new(
            manager,
            "referee-secret".to_string(),
            false,
        ));
        let dispatcher = interfaces::mcp::McpDispatcher::new(service);

        // 1. referee_query_rules
        let rules_res = dispatcher
            .dispatch("referee_query_rules", &json!({}))
            .expect("referee query rules");
        assert!(rules_res.is_object());
        let terrains = rules_res["terrains"].as_array().expect("terrains array");
        assert!(
            terrains
                .iter()
                .any(|t| t.get("name").and_then(Value::as_str) == Some("平地"))
        );
        let objects = rules_res["objects"].as_array().expect("objects array");
        assert!(
            objects
                .iter()
                .any(|o| o.get("name").and_then(Value::as_str) == Some("墙体"))
        );

        // 2. referee_create_room (3 bots)
        let create_res = dispatcher
            .dispatch(
                "referee_create_room",
                &json!({
                    "room_name": "裁判演练房",
                    "initial_bots": 3
                }),
            )
            .expect("referee create room");
        let room_id = create_res["id"].as_str().expect("id string").to_string();
        assert_eq!(create_res["phase"], "waiting");
        assert_eq!(create_res["members"].as_array().unwrap().len(), 3);
        assert_eq!(create_res["referee_managed"], true);

        // 3. referee_setup_bots (add bot -> remove bot)
        let add_bot_res = dispatcher
            .dispatch(
                "referee_setup_bots",
                &json!({
                    "room_id": room_id,
                    "action": "add",
                    "expected_revision": 0
                }),
            )
            .expect("referee add bot");
        assert_eq!(add_bot_res["members"].as_array().unwrap().len(), 4);

        let remove_bot_res = dispatcher
            .dispatch(
                "referee_setup_bots",
                &json!({
                    "room_id": room_id,
                    "action": "remove",
                    "expected_revision": 0
                }),
            )
            .expect("referee remove bot");
        assert_eq!(remove_bot_res["members"].as_array().unwrap().len(), 3);

        // 4. referee_start_match with deterministic seed and idempotency key
        let start_res = dispatcher
            .dispatch(
                "referee_start_match",
                &json!({
                    "room_id": room_id,
                    "seed": 424_242,
                    "expected_revision": 0,
                    "idempotency_key": "start-match-key-1"
                }),
            )
            .expect("referee start match");
        assert_eq!(start_res["phase"], "playing");
        let game = start_res["game"].as_object().expect("game object");
        let mut match_rev = game["revision"].as_u64().unwrap();
        assert_eq!(match_rev, 0);

        // Test idempotency: replaying same start match returns cached result
        let start_replay = dispatcher
            .dispatch(
                "referee_start_match",
                &json!({
                    "room_id": room_id,
                    "seed": 424_242,
                    "expected_revision": 0,
                    "idempotency_key": "start-match-key-1"
                }),
            )
            .expect("start match idempotency replay");
        assert_eq!(start_replay["phase"], "playing");

        // 5. referee_inspect_room (omniscient visibility)
        let inspect_res = dispatcher
            .dispatch(
                "referee_inspect_room",
                &json!({
                    "room_id": room_id
                }),
            )
            .expect("referee inspect room");
        let inspected_game = inspect_res.get("game").expect("omniscient game present");
        let players = inspected_game["players"].as_array().expect("players array");
        assert_eq!(players.len(), 3);
        for player in players {
            assert!(player.get("position").is_some());
            assert_eq!(player.get("status").and_then(Value::as_str), Some("alive"));
            assert!(player.get("has_shield").is_some());
        }

        // 6. referee_step_bot
        let step_res = dispatcher
            .dispatch(
                "referee_step_bot",
                &json!({
                    "room_id": room_id,
                    "expected_revision": match_rev,
                    "idempotency_key": "step-key-1"
                }),
            )
            .expect("referee step bot");
        assert!(!step_res["action_desc"].as_str().unwrap().is_empty());
        match_rev = step_res["new_revision"].as_u64().unwrap();
        assert_eq!(match_rev, 1);

        // Test idempotency on bot step
        let step_replay = dispatcher
            .dispatch(
                "referee_step_bot",
                &json!({
                    "room_id": room_id,
                    "expected_revision": 999,
                    "idempotency_key": "step-key-1"
                }),
            )
            .expect("step bot idempotency replay");
        assert_eq!(step_replay["new_revision"].as_u64().unwrap(), match_rev);

        // Test OCC revision conflict: wrong revision without idempotency fails
        let conflict_err = dispatcher
            .dispatch(
                "referee_step_bot",
                &json!({
                    "room_id": room_id,
                    "expected_revision": 0
                }),
            )
            .unwrap_err();
        assert!(conflict_err.message.contains("revision conflict"));

        // 7. referee_force_command
        let force_res = dispatcher
            .dispatch(
                "referee_force_command",
                &json!({
                    "room_id": room_id,
                    "expected_revision": match_rev,
                    "command": { "type": "wait" }
                }),
            )
            .expect("referee force command");
        assert!(!force_res["action_desc"].as_str().unwrap().is_empty());
        let final_rev = force_res["new_revision"].as_u64().unwrap();
        assert!(final_rev > match_rev);

        // 8. referee_render_room_image (SVG and PNG test)
        let render_svg = dispatcher
            .dispatch(
                "referee_render_room_image",
                &json!({
                    "room_id": room_id,
                    "format": "svg"
                }),
            )
            .expect("referee render room svg");
        assert_eq!(render_svg["format"], "svg");
        assert!(render_svg["svg"].as_str().unwrap().contains("<svg"));

        let render_png = dispatcher
            .dispatch(
                "referee_render_room_image",
                &json!({
                    "room_id": room_id,
                    "format": "png"
                }),
            )
            .expect("referee render room png");
        assert_eq!(render_png["format"], "png");
        assert_eq!(render_png["content_type"], "image/png");
        assert!(
            render_png["data_uri"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert!(render_png["size_bytes"].as_u64().unwrap() > 1024);

        // 8b. referee_render_room_image (standalone board view)
        let render_board_svg = dispatcher
            .dispatch(
                "referee_render_room_image",
                &json!({
                    "room_id": room_id,
                    "format": "svg",
                    "view_mode": "board"
                }),
            )
            .expect("referee render room board svg");
        assert_eq!(render_board_svg["view_mode"], "board");
        assert!(
            render_board_svg["svg"]
                .as_str()
                .unwrap()
                .contains("terrain-layer")
        );

        let render_board_png = dispatcher
            .dispatch(
                "referee_render_room_image",
                &json!({
                    "room_id": room_id,
                    "format": "png",
                    "view_mode": "board"
                }),
            )
            .expect("referee render room board png");
        assert_eq!(render_board_png["view_mode"], "board");
        assert!(render_board_png["size_bytes"].as_u64().unwrap() > 1024);

        // 8c. referee_render_asset_sheet
        let sheet_svg = dispatcher
            .dispatch("referee_render_asset_sheet", &json!({ "format": "svg" }))
            .expect("referee render asset sheet svg");
        assert!(
            sheet_svg["svg"]
                .as_str()
                .unwrap()
                .contains("PROCEDURAL VECTOR ASSET CATALOG")
        );

        let sheet_png = dispatcher
            .dispatch("referee_render_asset_sheet", &json!({ "format": "png" }))
            .expect("referee render asset sheet png");
        assert!(sheet_png["size_bytes"].as_u64().unwrap() > 5000);

        // 9. referee_list_rooms
        let list_res = dispatcher
            .dispatch("referee_list_rooms", &json!({}))
            .expect("referee list rooms");
        let list_array = list_res.as_array().unwrap();
        assert_eq!(list_array.len(), 1);
        assert_eq!(list_array[0]["id"], room_id);

        // 9. referee_dissolve_room
        let dissolve_res = dispatcher
            .dispatch(
                "referee_dissolve_room",
                &json!({
                    "room_id": room_id
                }),
            )
            .expect("referee dissolve room");
        assert_eq!(dissolve_res["room_id"], room_id);
        assert!(dissolve_res.get("message").is_some());

        // Verify room is gone
        let list_after = dispatcher
            .dispatch("referee_list_rooms", &json!({}))
            .expect("referee list rooms after dissolve");
        assert!(list_after.as_array().unwrap().is_empty());
    }
}
