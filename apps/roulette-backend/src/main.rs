//! Local multi-room Web composition root.

#![forbid(unsafe_code)]

use std::{
    collections::HashSet,
    env,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use roulette_domain::{LobbyView, PlayerCommand, RoomMemberId, RoomView, TabId};
use roulette_host::{CreateRoomConfig, HostError, JoinRoomConfig, LocalLobby};
use serde::{Deserialize, Serialize};

const INDEX_HTML: &str = include_str!("../../../clients/web/index.html");
const STYLES_CSS: &str = include_str!("../../../clients/web/styles.css");
const APP_JS: &str = include_str!("../../../clients/web/app.js");
const TAB_TIMEOUT: Duration = Duration::from_secs(15);

struct AppState {
    lobby: Mutex<LocalLobby>,
    founder_password: String,
    founder_tabs: Mutex<HashSet<TabId>>,
    lan_exposed: AtomicBool,
}

#[derive(Deserialize)]
struct TabQuery {
    tab_id: TabId,
}

#[derive(Deserialize)]
struct TabRequest {
    tab_id: TabId,
}

#[derive(Deserialize)]
struct CreateRoomRequest {
    tab_id: TabId,
    room_name: String,
    player_name: String,
    password: Option<String>,
}

#[derive(Deserialize)]
struct JoinRoomRequest {
    tab_id: TabId,
    room_id: String,
    player_name: String,
    password: Option<String>,
}

#[derive(Deserialize)]
struct MemberRequest {
    tab_id: TabId,
    member_id: RoomMemberId,
}

#[derive(Deserialize)]
struct StartRoomRequest {
    tab_id: TabId,
    seed: Option<u64>,
}

#[derive(Deserialize)]
struct CommandRequest {
    tab_id: TabId,
    expected_revision: u64,
    command: PlayerCommand,
}

#[derive(Deserialize)]
struct StepRequest {
    tab_id: TabId,
    expected_revision: u64,
}

#[derive(Deserialize)]
struct FounderAuthRequest {
    tab_id: TabId,
    password: String,
}

#[derive(Deserialize)]
struct ServerSettingsRequest {
    tab_id: TabId,
    expose_lan: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct FounderAuthResponse {
    authenticated: bool,
}

#[derive(Serialize)]
struct ServerSettingsResponse {
    founder_authenticated: bool,
    lan_exposed: bool,
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
    fn from_host(error: &HostError) -> Self {
        let status = match error {
            HostError::RoomNotFound => StatusCode::NOT_FOUND,
            HostError::NotRoomMember
            | HostError::IncorrectRoomPassword
            | HostError::OwnerRequired
            | HostError::HumanDoesNotOwnTurn => StatusCode::FORBIDDEN,
            HostError::InvalidInput(_) | HostError::InvalidMemberCount => StatusCode::BAD_REQUEST,
            HostError::AlreadyInRoom
            | HostError::RoomFull
            | HostError::RoomNotWaiting
            | HostError::InvalidMember
            | HostError::MatchNotRunning
            | HostError::RevisionConflict { .. }
            | HostError::BotDoesNotOwnTurn => StatusCode::CONFLICT,
            HostError::AutomationLimitExceeded | HostError::Core(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        Self {
            status,
            message: error.to_string(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
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
    let host = env::var("ROULETTE_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
    let port = env::var("ROULETTE_HTTP_PORT")
        .unwrap_or_else(|_| "8787".to_owned())
        .parse::<u16>()?;
    let address = SocketAddr::new(host.parse::<IpAddr>()?, port);
    let founder_password = configured_founder_password();
    let lan_exposed = env::var("ROULETTE_LAN_EXPOSED")
        .is_ok_and(|value| value.eq_ignore_ascii_case("true") || value == "1");
    let state = Arc::new(AppState {
        lobby: Mutex::new(LocalLobby::new(random_seed())),
        founder_password: founder_password.clone(),
        founder_tabs: Mutex::new(HashSet::new()),
        lan_exposed: AtomicBool::new(lan_exposed),
    });

    let app = build_router(Arc::clone(&state));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("俄罗斯轮盘本地大厅：http://127.0.0.1:{port}");
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

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/styles.css", get(styles))
        .route("/app.js", get(script))
        .route("/api/health", get(health))
        .route("/api/lobby", get(lobby_view))
        .route("/api/session/heartbeat", post(heartbeat))
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/join", post(join_room))
        .route("/api/rooms/{room_id}", get(room_view))
        .route("/api/rooms/{room_id}/leave", post(leave_room))
        .route("/api/rooms/{room_id}/bots/add", post(add_bot))
        .route("/api/rooms/{room_id}/bots/remove", post(remove_bot))
        .route("/api/rooms/{room_id}/owner", post(transfer_owner))
        .route("/api/rooms/{room_id}/start", post(start_room))
        .route("/api/rooms/{room_id}/dissolve", post(dissolve_room))
        .route("/api/rooms/{room_id}/command", post(command))
        .route("/api/rooms/{room_id}/step", post(step))
        .route("/api/founder/auth", post(founder_auth))
        .route(
            "/api/server/settings",
            get(server_settings).post(update_server_settings),
        )
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            lan_guard,
        ))
        .with_state(state)
}

async fn lan_guard(
    State(state): State<Arc<AppState>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if remote.ip().is_loopback() || state.lan_exposed.load(Ordering::Relaxed) {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "局域网访问尚未由本机创始人开启").into_response()
    }
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

async fn lobby_view(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TabQuery>,
) -> Result<Json<LobbyView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &query.tab_id);
    Ok(Json(lobby.view(&query.tab_id)))
}

async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TabRequest>,
) -> Result<StatusCode, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &request.tab_id);
    Ok(StatusCode::NO_CONTENT)
}

async fn create_room(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    let view = lobby
        .create_room(
            &CreateRoomConfig {
                tab_id: request.tab_id,
                room_name: request.room_name,
                player_name: request.player_name,
                password: request.password,
            },
            Instant::now(),
        )
        .map_err(|error| ApiError::from_host(&error))?;
    Ok(Json(view))
}

async fn join_room(
    State(state): State<Arc<AppState>>,
    Json(request): Json<JoinRoomRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    lobby.reap_inactive(Instant::now(), TAB_TIMEOUT);
    let view = lobby
        .join_room(
            &JoinRoomConfig {
                tab_id: request.tab_id,
                room_id: request.room_id,
                player_name: request.player_name,
                password: request.password,
            },
            Instant::now(),
        )
        .map_err(|error| ApiError::from_host(&error))?;
    Ok(Json(view))
}

async fn room_view(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Query(query): Query<TabQuery>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &query.tab_id);
    lobby
        .room_view(&room_id, &query.tab_id)
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

async fn leave_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<TabRequest>,
) -> Result<Json<LobbyView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    lobby
        .leave_room(&room_id, &request.tab_id)
        .map_err(|error| ApiError::from_host(&error))?;
    Ok(Json(lobby.view(&request.tab_id)))
}

async fn add_bot(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<TabRequest>,
) -> Result<Json<RoomView>, ApiError> {
    mutate_room(&state, &room_id, &request.tab_id, LocalLobby::add_bot)
}

async fn remove_bot(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<MemberRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &request.tab_id);
    lobby
        .remove_bot(&room_id, &request.tab_id, &request.member_id)
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

async fn transfer_owner(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<MemberRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &request.tab_id);
    lobby
        .transfer_owner(&room_id, &request.tab_id, &request.member_id)
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

async fn start_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<StartRoomRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &request.tab_id);
    lobby
        .start_room(
            &room_id,
            &request.tab_id,
            request.seed.unwrap_or_else(random_seed),
        )
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

async fn dissolve_room(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<TabRequest>,
) -> Result<Json<LobbyView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    lobby
        .dissolve_room(&room_id, &request.tab_id)
        .map_err(|error| ApiError::from_host(&error))?;
    Ok(Json(lobby.view(&request.tab_id)))
}

async fn command(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &request.tab_id);
    lobby
        .submit_command(
            &room_id,
            &request.tab_id,
            request.expected_revision,
            request.command,
        )
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

async fn step(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Json(request): Json<StepRequest>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(&state)?;
    refresh_lobby(&mut lobby, &request.tab_id);
    lobby
        .step_bot(&room_id, &request.tab_id, request.expected_revision)
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

async fn founder_auth(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FounderAuthRequest>,
) -> Result<Json<FounderAuthResponse>, ApiError> {
    if request.password != state.founder_password {
        return Err(ApiError::forbidden("创始人临时密码错误"));
    }
    state
        .founder_tabs
        .lock()
        .map_err(|_| ApiError::internal("创始人会话锁已损坏"))?
        .insert(request.tab_id);
    Ok(Json(FounderAuthResponse {
        authenticated: true,
    }))
}

async fn server_settings(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TabQuery>,
) -> Result<Json<ServerSettingsResponse>, ApiError> {
    Ok(Json(settings_response(&state, &query.tab_id)?))
}

async fn update_server_settings(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ServerSettingsRequest>,
) -> Result<Json<ServerSettingsResponse>, ApiError> {
    if !is_founder(&state, &request.tab_id)? {
        return Err(ApiError::forbidden("请先验证创始人临时密码"));
    }
    state
        .lan_exposed
        .store(request.expose_lan, Ordering::Relaxed);
    Ok(Json(settings_response(&state, &request.tab_id)?))
}

fn mutate_room(
    state: &AppState,
    room_id: &str,
    tab_id: &TabId,
    operation: impl FnOnce(&mut LocalLobby, &str, &TabId) -> Result<RoomView, HostError>,
) -> Result<Json<RoomView>, ApiError> {
    let mut lobby = lock_lobby(state)?;
    refresh_lobby(&mut lobby, tab_id);
    operation(&mut lobby, room_id, tab_id)
        .map(Json)
        .map_err(|error| ApiError::from_host(&error))
}

fn lock_lobby(state: &AppState) -> Result<std::sync::MutexGuard<'_, LocalLobby>, ApiError> {
    state
        .lobby
        .lock()
        .map_err(|_| ApiError::internal("大厅状态锁已损坏"))
}

fn refresh_lobby(lobby: &mut LocalLobby, tab_id: &TabId) {
    let now = Instant::now();
    lobby.reap_inactive(now, TAB_TIMEOUT);
    lobby.heartbeat(tab_id, now);
}

fn is_founder(state: &AppState, tab_id: &TabId) -> Result<bool, ApiError> {
    Ok(state
        .founder_tabs
        .lock()
        .map_err(|_| ApiError::internal("创始人会话锁已损坏"))?
        .contains(tab_id))
}

fn settings_response(state: &AppState, tab_id: &TabId) -> Result<ServerSettingsResponse, ApiError> {
    Ok(ServerSettingsResponse {
        founder_authenticated: is_founder(state, tab_id)?,
        lan_exposed: state.lan_exposed.load(Ordering::Relaxed),
    })
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
        .unwrap_or_else(|| format!("{:08X}", random_seed() & u64::from(u32::MAX)))
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
