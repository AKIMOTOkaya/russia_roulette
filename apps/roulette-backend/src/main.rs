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
    println!("MCP Streamable 端点：http://127.0.0.1:{port}/mcp");
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
        assert!(rules_res.is_array());
        let rules_list = rules_res.as_array().unwrap();
        assert!(!rules_list.is_empty());
        assert!(
            rules_list
                .iter()
                .any(|t| t.get("name").and_then(Value::as_str) == Some("墙体"))
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
                    "expected_revision": 999, // wrong revision but matching idempotency key returns cache
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

        // 8. referee_list_rooms
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
