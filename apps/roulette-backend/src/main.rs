//! Local Web MVP composition root.

#![forbid(unsafe_code)]

use std::{
    env,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use roulette_domain::{GameView, PlayerCommand};
use roulette_host::{HostError, LocalMatch, LocalMatchConfig};
use serde::{Deserialize, Serialize};

const INDEX_HTML: &str = include_str!("../../../clients/web/index.html");
const STYLES_CSS: &str = include_str!("../../../clients/web/styles.css");
const APP_JS: &str = include_str!("../../../clients/web/app.js");

#[derive(Default)]
struct AppState {
    game: Mutex<Option<LocalMatch>>,
}

#[derive(Deserialize)]
struct NewGameRequest {
    player_name: String,
    bot_count: u8,
    seed: Option<u64>,
}

#[derive(Deserialize)]
struct CommandRequest {
    expected_revision: u64,
    command: PlayerCommand,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn conflict(error: &HostError) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: error.to_string(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("ROULETTE_HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("ROULETTE_HTTP_PORT")
        .unwrap_or_else(|_| "8787".to_owned())
        .parse::<u16>()?;
    let address = SocketAddr::new(host.parse::<IpAddr>()?, port);

    let app = build_router(Arc::new(AppState::default()));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("俄罗斯轮盘本地版已启动：http://{address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/styles.css", get(styles))
        .route("/app.js", get(script))
        .route("/api/health", get(health))
        .route("/api/game", get(get_game))
        .route("/api/game/new", post(new_game))
        .route("/api/game/command", post(command))
        .with_state(state)
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    )
}

async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        APP_JS,
    )
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn get_game(State(state): State<Arc<AppState>>) -> Result<Json<GameView>, ApiError> {
    let game = state
        .game
        .lock()
        .map_err(|_| ApiError::internal("对局状态锁已损坏"))?;
    let game = game.as_ref().ok_or_else(|| ApiError {
        status: StatusCode::NOT_FOUND,
        message: "尚未创建对局".to_owned(),
    })?;
    game.view()
        .map(Json)
        .map_err(|error| ApiError::conflict(&error))
}

async fn new_game(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NewGameRequest>,
) -> Result<Json<GameView>, ApiError> {
    let player_name = request.player_name.trim();
    if player_name.is_empty() || player_name.chars().count() > 24 {
        return Err(ApiError {
            status: StatusCode::BAD_REQUEST,
            message: "玩家名长度必须为 1～24 个字符".to_owned(),
        });
    }

    let game = LocalMatch::new(&LocalMatchConfig {
        human_name: player_name.to_owned(),
        bot_count: request.bot_count,
        seed: request.seed.unwrap_or_else(random_seed),
    })
    .map_err(|error| ApiError::conflict(&error))?;
    let view = game.view().map_err(|error| ApiError::conflict(&error))?;
    *state
        .game
        .lock()
        .map_err(|_| ApiError::internal("对局状态锁已损坏"))? = Some(game);
    Ok(Json(view))
}

async fn command(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<GameView>, ApiError> {
    let mut game = state
        .game
        .lock()
        .map_err(|_| ApiError::internal("对局状态锁已损坏"))?;
    let game = game.as_mut().ok_or_else(|| ApiError {
        status: StatusCode::NOT_FOUND,
        message: "尚未创建对局".to_owned(),
    })?;
    game.submit_human_command(request.expected_revision, request.command)
        .map(Json)
        .map_err(|error| ApiError::conflict(&error))
}

fn random_seed() -> u64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_secs() ^ u64::from(duration.subsec_nanos())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
