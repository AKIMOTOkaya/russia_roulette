//! Local multi-room Web and MCP composition root.

#![forbid(unsafe_code)]

mod interfaces;

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use roulette_host::{GameManager, GameService, InMemoryGameRepository, generate_seed};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("ROULETTE_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
    let port = env::var("ROULETTE_HTTP_PORT")
        .unwrap_or_else(|_| "8787".to_owned())
        .parse::<u16>()?;
    let address = SocketAddr::new(host.parse::<IpAddr>()?, port);
    let founder_password = configured_founder_password();
    let lan_exposed = env::var("ROULETTE_LAN_EXPOSED")
        .is_ok_and(|value| value.eq_ignore_ascii_case("true") || value == "1");

    // 1. Persistence Layer: Repository (InMemory default, prepared for SQLite / PostgreSQL)
    let repository = Arc::new(InMemoryGameRepository::new());

    // 2. Domain Management Layer: GameManager
    let manager = Arc::new(GameManager::new(repository, generate_seed()));

    // 3. Application Service Layer: GameService
    let service = Arc::new(GameService::new(
        manager,
        founder_password.clone(),
        lan_exposed,
    ));

    // 4. Interface Layer: HTTP + MCP
    let app = interfaces::http::build_router(Arc::clone(&service));

    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("俄罗斯轮盘本地服务：http://127.0.0.1:{port}");
    println!("MCP 工具端点：http://127.0.0.1:{port}/api/mcp/tools");
    println!("创始人临时密码：{founder_password}");
    println!(
        "局域网访问：{}",
        if lan_exposed {
            "已开启"
        } else {
            "默认关闭"
        }
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
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

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
