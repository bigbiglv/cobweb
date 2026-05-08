use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, DefaultBodyLimit, Multipart, Path, Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use cobweb_scheduler::{ScheduledTask, TaskOrigin};
use cobweb_service::{
    get_feature_groups, get_feature_snapshot, FeatureCommand, FeatureExecutionResult, FeatureGroup,
    FeatureSnapshot,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap as StdHashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::{
    clipboard_sync::{
        self, ClipboardSyncMessage, ClipboardSyncSource, ClipboardSyncSourceKind, IncomingAttachment,
    },
    features::execute_feature_command_with_origin,
    scheduler,
    store::{PairedClient, TaskHistoryEntry, GLOBAL_STORE},
    web_console,
};

lazy_static! {
    pub static ref PENDING_REQUESTS: Arc<Mutex<StdHashMap<String, oneshot::Sender<bool>>>> =
        Arc::new(Mutex::new(StdHashMap::new()));
    pub static ref ONLINE_CLIENTS: Arc<Mutex<HashSet<String>>> =
        Arc::new(Mutex::new(HashSet::new()));
    pub static ref ACTIVE_CONNECTIONS: Arc<Mutex<HashSet<String>>> =
        Arc::new(Mutex::new(HashSet::new()));
    pub static ref LAST_HEARTBEATS: Arc<Mutex<StdHashMap<String, u64>>> =
        Arc::new(Mutex::new(StdHashMap::new()));
    static ref WS_CONNECTIONS: Arc<Mutex<StdHashMap<String, WsConnectionHandle>>> =
        Arc::new(Mutex::new(StdHashMap::new()));
    static ref WEB_CONNECTIONS: Arc<Mutex<StdHashMap<String, mpsc::UnboundedSender<WebServerEvent>>>> =
        Arc::new(Mutex::new(StdHashMap::new()));
    static ref WEB_SERVER_PORT: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
}

#[derive(Clone)]
struct WsConnectionHandle {
    connection_id: String,
    sender: mpsc::UnboundedSender<WsServerEvent>,
}

#[derive(Clone)]
pub struct AppState {
    pub tauri_app: AppHandle,
}

#[derive(Clone)]
struct StoredClientSnapshot {
    client_id: String,
    client_name: String,
    last_seen_at: u64,
    last_ip: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PairRequestInfo {
    pub client_id: String,
    pub client_name: String,
}

#[derive(Serialize)]
pub struct PairResponse {
    pub success: bool,
    pub token: Option<String>,
    pub msg: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub client_id: String,
    pub token: String,
    pub client_name: Option<String>,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub msg: String,
}

#[derive(Deserialize)]
pub struct SessionRequest {
    pub client_id: String,
    pub token: String,
    pub client_name: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SessionEvent {
    pub client_id: String,
    pub client_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebClientInfo {
    pub client_id: Option<String>,
    pub device_name: Option<String>,
    pub device_model: Option<String>,
    pub platform: Option<String>,
    pub browser: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Deserialize)]
struct WsSessionQuery {
    client_id: String,
    token: String,
    client_name: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsServerEvent {
    SessionReady {
        client_id: String,
        client_name: String,
    },
    TasksSync {
        tasks: Vec<ScheduledTask>,
    },
    Disconnect {
        reason: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsClientEvent {
    Heartbeat,
    RequestTasksSync,
    Disconnect,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WebServerEvent {
    StateSync {
        groups: Vec<FeatureGroup>,
        snapshot: Option<FeatureSnapshot>,
        tasks: Vec<ScheduledTask>,
        history: Vec<TaskHistoryEntry>,
        #[serde(rename = "syncMessages")]
        sync_messages: Vec<ClipboardSyncMessage>,
    },
    ClipboardSync {
        messages: Vec<ClipboardSyncMessage>,
    },
    FeatureResult {
        request_id: String,
        success: bool,
        msg: String,
        result: Option<FeatureExecutionResult>,
    },
    Pong,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WebClientEvent {
    Heartbeat,
    RequestStateSync {
        #[serde(default)]
        client_info: Option<WebClientInfo>,
    },
    ExecuteFeature {
        request_id: String,
        #[serde(default)]
        client_info: Option<WebClientInfo>,
        #[serde(flatten)]
        command: FeatureCommand,
    },
    Disconnect,
}

#[derive(Deserialize)]
pub struct MobileFeatureCatalogRequest {
    pub client_id: String,
    pub token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureCatalogResponse {
    pub success: bool,
    pub msg: String,
    pub groups: Vec<FeatureGroup>,
    pub snapshot: Option<FeatureSnapshot>,
}

#[derive(Deserialize)]
pub struct MobileFeatureExecuteRequest {
    pub client_id: String,
    pub token: String,
    #[serde(flatten)]
    pub command: FeatureCommand,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureExecuteResponse {
    pub success: bool,
    pub msg: String,
    pub result: Option<FeatureExecutionResult>,
}

#[derive(Deserialize)]
pub struct MobileTaskListRequest {
    pub client_id: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct MobileTaskCreateRequest {
    pub client_id: String,
    pub token: String,
    pub execute_at_ms: u64,
    #[serde(flatten)]
    pub command: FeatureCommand,
}

#[derive(Deserialize)]
pub struct MobileTaskCancelRequest {
    pub client_id: String,
    pub token: String,
    pub task_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResponse {
    pub success: bool,
    pub msg: String,
    pub tasks: Vec<ScheduledTask>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCreateResponse {
    pub success: bool,
    pub msg: String,
    pub task: Option<ScheduledTask>,
}

#[derive(Deserialize)]
pub struct WebFeatureExecuteRequest {
    #[serde(default)]
    pub client_info: Option<WebClientInfo>,
    #[serde(flatten)]
    pub command: FeatureCommand,
}

#[derive(Deserialize)]
pub struct WebTaskCreateRequest {
    pub execute_at_ms: u64,
    #[serde(default)]
    pub client_info: Option<WebClientInfo>,
    #[serde(flatten)]
    pub command: FeatureCommand,
}

#[derive(Deserialize)]
pub struct WebTaskCancelRequest {
    #[serde(default)]
    pub client_info: Option<WebClientInfo>,
    pub task_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebStateResponse {
    pub success: bool,
    pub msg: String,
    pub groups: Vec<FeatureGroup>,
    pub snapshot: Option<FeatureSnapshot>,
    pub tasks: Vec<ScheduledTask>,
    pub history: Vec<TaskHistoryEntry>,
    pub sync_messages: Vec<ClipboardSyncMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSyncResponse {
    pub success: bool,
    pub msg: String,
    pub message: Option<ClipboardSyncMessage>,
    pub messages: Vec<ClipboardSyncMessage>,
}

#[derive(Serialize, Clone)]
pub struct ClientInfo {
    pub client_id: String,
    pub client_name: String,
    pub last_seen_at: u64,
    pub last_ip: Option<String>,
    pub is_online: bool,
    pub is_connected: bool,
}

pub fn current_web_port() -> Option<u16> {
    *WEB_SERVER_PORT.lock().unwrap()
}

pub async fn start_server(port: u16, tauri_app: AppHandle) -> Result<u16, String> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let state = AppState { tauri_app };

    let mobile_routes = Router::new()
        .route("/auth/pair", post(auth_pair))
        .route("/auth/verify", post(auth_verify))
        .route("/auth/connect", post(auth_connect))
        .route("/auth/disconnect", post(auth_disconnect))
        .route("/auth/heartbeat", post(auth_heartbeat))
        .route("/ws/session", get(ws_session))
        .route("/features/catalog", post(features_catalog))
        .route("/features/execute", post(features_execute))
        .route("/tasks/list", post(tasks_list))
        .route("/tasks/create", post(tasks_create))
        .route("/tasks/cancel", post(tasks_cancel))
        .layer(cors.clone());

    let web_routes = Router::new()
        .route("/web", get(web_index))
        .route("/web/", get(web_index))
        .route("/web/assets/style.css", get(web_css))
        .route("/web/assets/main.js", get(web_main_js))
        .route("/web/ws", get(web_ws))
        .route("/web/api/state", get(web_state))
        .route("/web/api/features/execute", post(web_features_execute))
        .route("/web/api/tasks/create", post(web_tasks_create))
        .route("/web/api/tasks/cancel", post(web_tasks_cancel))
        .route("/web/api/sync/history", get(web_clipboard_sync_history))
        .route("/web/api/sync/messages", post(web_clipboard_sync_create))
        .route(
            "/web/api/sync/files/:message_id/:attachment_id",
            get(web_clipboard_sync_file),
        )
        .layer(DefaultBodyLimit::max(200 * 1024 * 1024))
        .layer(cors);

    let app = Router::new()
        .merge(mobile_routes)
        .merge(web_routes)
        .with_state(state)
        .into_make_service_with_connect_info::<SocketAddr>();

    let mut listener = None;
    let mut selected_port = port;

    for candidate_port in port..=u16::MAX {
        let addr = SocketAddr::from(([0, 0, 0, 0], candidate_port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(candidate_listener) => {
                listener = Some(candidate_listener);
                selected_port = candidate_port;
                break;
            }
            Err(error) => {
                log::warn!(
                    "Failed to bind Axum to port {}: {}. Trying next port.",
                    candidate_port,
                    error
                );
            }
        }
    }

    let Some(listener) = listener else {
        return Err(format!("No available port found from {} to {}", port, u16::MAX));
    };

    {
        let mut web_server_port = WEB_SERVER_PORT.lock().unwrap();
        *web_server_port = Some(selected_port);
    }

    log::info!("Axum server listening on 0.0.0.0:{}", selected_port);

    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            log::error!("Axum server error: {}", error);
        }
    });

    Ok(selected_port)
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn is_lan_or_loopback(addr: &SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        std::net::IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unique_local() || ip.segments()[0] & 0xffc0 == 0xfe80
        }
    }
}

fn reject_non_lan(addr: &SocketAddr) -> Option<impl IntoResponse> {
    if is_lan_or_loopback(addr) {
        None
    } else {
        Some((
            StatusCode::FORBIDDEN,
            "Web console is only available on the local network",
        ))
    }
}

fn web_state_payload() -> WebStateResponse {
    WebStateResponse {
        success: true,
        msg: "OK".into(),
        groups: get_feature_groups(),
        snapshot: get_feature_snapshot().ok(),
        tasks: scheduler::list_tasks(),
        history: scheduler::list_task_history(),
        sync_messages: clipboard_sync::list_messages().unwrap_or_default(),
    }
}

fn web_state_event() -> WebServerEvent {
    let state = web_state_payload();
    WebServerEvent::StateSync {
        groups: state.groups,
        snapshot: state.snapshot,
        tasks: state.tasks,
        history: state.history,
        sync_messages: state.sync_messages,
    }
}

fn emit_clients_changed(app: &AppHandle) {
    let _ = app.emit("paired_clients_changed", serde_json::json!({}));
}

fn emit_session_event(app: &AppHandle, event_name: &str, event: &SessionEvent) {
    let _ = app.emit(event_name, event.clone());
}

fn get_client_name(client_id: &str) -> String {
    let store = GLOBAL_STORE.lock().unwrap();
    store
        .data
        .paired_clients
        .get(client_id)
        .map(|client| client.client_name.clone())
        .unwrap_or_else(|| "未知设备".to_string())
}

fn clean_web_client_part(value: Option<&str>) -> Option<String> {
    let cleaned = value
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .chars()
        .filter(|ch| !ch.is_control())
        .take(64)
        .collect::<String>();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn fallback_browser_name(user_agent: Option<&str>) -> Option<String> {
    let user_agent = user_agent?;
    if user_agent.contains("Edg/") {
        Some("Edge".into())
    } else if user_agent.contains("Chrome/") || user_agent.contains("CriOS/") {
        Some("Chrome".into())
    } else if user_agent.contains("Firefox/") || user_agent.contains("FxiOS/") {
        Some("Firefox".into())
    } else if user_agent.contains("Safari/") {
        Some("Safari".into())
    } else {
        None
    }
}

fn fallback_platform_name(user_agent: Option<&str>) -> Option<String> {
    let user_agent = user_agent?;
    if user_agent.contains("iPhone") {
        Some("iPhone".into())
    } else if user_agent.contains("iPad") {
        Some("iPad".into())
    } else if user_agent.contains("Android") {
        Some("Android".into())
    } else if user_agent.contains("Windows") {
        Some("Windows".into())
    } else if user_agent.contains("Mac OS X") {
        Some("macOS".into())
    } else {
        None
    }
}

fn web_origin(addr: &SocketAddr, client_info: Option<&WebClientInfo>) -> TaskOrigin {
    let client_id = client_info
        .and_then(|info| clean_web_client_part(info.client_id.as_deref()))
        .unwrap_or_else(|| format!("web-{}", addr.ip()));
    let user_agent = client_info.and_then(|info| info.user_agent.as_deref());
    let device_name =
        client_info.and_then(|info| clean_web_client_part(info.device_name.as_deref()));
    let device_model =
        client_info.and_then(|info| clean_web_client_part(info.device_model.as_deref()));
    let platform = client_info
        .and_then(|info| clean_web_client_part(info.platform.as_deref()))
        .or_else(|| fallback_platform_name(user_agent));
    let browser = client_info
        .and_then(|info| clean_web_client_part(info.browser.as_deref()))
        .or_else(|| fallback_browser_name(user_agent));

    let mut parts = Vec::new();
    if let Some(value) = device_name.or(device_model) {
        parts.push(value);
    }
    if let Some(value) = platform {
        if !parts.iter().any(|part| part == &value) {
            parts.push(value);
        }
    }
    if let Some(value) = browser {
        parts.push(value);
    }

    let client_name = if parts.is_empty() {
        format!("Web 控制台 {}", addr.ip())
    } else {
        format!("{} · Web 控制台 {}", parts.join(" "), addr.ip())
    };

    TaskOrigin::web_client(client_id, client_name)
}

fn emit_web_notice(app: &AppHandle, origin: &TaskOrigin, message: String) {
    let _ = app.emit(
        "feature_notice",
        serde_json::json!({
            "title": "Web 指令",
            "message": format!("{} {}", origin.client_name, message),
            "tone": "warning",
        }),
    );
}

fn emit_clipboard_sync_changed(app: &AppHandle) {
    let _ = app.emit("clipboard_sync_changed", serde_json::json!({}));
}

fn clipboard_source(
    addr: &SocketAddr,
    source_kind: ClipboardSyncSourceKind,
    client_info: Option<&WebClientInfo>,
) -> ClipboardSyncSource {
    match source_kind {
        ClipboardSyncSourceKind::Pc => {
            let store = GLOBAL_STORE.lock().unwrap();
            ClipboardSyncSource {
                kind: ClipboardSyncSourceKind::Pc,
                client_id: Some(store.data.device_id.clone()),
                device_name: Some(store.data.device_name.clone()),
                device_model: None,
                platform: Some(std::env::consts::OS.to_string()),
                browser: None,
                ip: Some(addr.ip().to_string()),
            }
        }
        ClipboardSyncSourceKind::Web => {
            let user_agent = client_info.and_then(|info| info.user_agent.as_deref());
            ClipboardSyncSource {
                kind: ClipboardSyncSourceKind::Web,
                client_id: client_info
                    .and_then(|info| clean_web_client_part(info.client_id.as_deref())),
                device_name: client_info
                    .and_then(|info| clean_web_client_part(info.device_name.as_deref())),
                device_model: client_info
                    .and_then(|info| clean_web_client_part(info.device_model.as_deref())),
                platform: client_info
                    .and_then(|info| clean_web_client_part(info.platform.as_deref()))
                    .or_else(|| fallback_platform_name(user_agent)),
                browser: client_info
                    .and_then(|info| clean_web_client_part(info.browser.as_deref()))
                    .or_else(|| fallback_browser_name(user_agent)),
                ip: Some(addr.ip().to_string()),
            }
        }
    }
}

fn sync_error_response(status: StatusCode, msg: impl Into<String>) -> axum::response::Response {
    (
        status,
        Json(ClipboardSyncResponse {
            success: false,
            msg: msg.into(),
            message: None,
            messages: vec![],
        }),
    )
        .into_response()
}

fn sync_client_state(
    client_id: &str,
    token: &str,
    ip: Option<&str>,
    client_name: Option<&str>,
) -> Option<String> {
    let mut store = GLOBAL_STORE.lock().unwrap();
    let client = store.data.paired_clients.get_mut(client_id)?;
    if client.token_hash != token {
        return None;
    }

    if let Some(next_name) = client_name
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != client.client_name)
    {
        client.client_name = next_name.to_string();
    }

    if let Some(next_ip) = ip.filter(|value| !value.is_empty()) {
        client.last_ip = Some(next_ip.to_string());
    }

    client.last_seen_at = current_timestamp_ms();
    let client_name = client.client_name.clone();
    let _ = store.save();
    Some(client_name)
}

fn collect_client_snapshots() -> Vec<StoredClientSnapshot> {
    let store = GLOBAL_STORE.lock().unwrap();
    store
        .data
        .paired_clients
        .values()
        .map(|client| StoredClientSnapshot {
            client_id: client.client_id.clone(),
            client_name: client.client_name.clone(),
            last_seen_at: client.last_seen_at,
            last_ip: client.last_ip.clone(),
        })
        .collect()
}

fn is_client_authorized(client_id: &str, token: &str) -> bool {
    let store = GLOBAL_STORE.lock().unwrap();
    store.is_client_paired(client_id, token)
}

fn mark_client_online(app: &AppHandle, client_id: &str) -> bool {
    let now = current_timestamp_ms();
    let became_online = {
        let mut online_clients = ONLINE_CLIENTS.lock().unwrap();
        online_clients.insert(client_id.to_string())
    };

    {
        let mut heartbeats = LAST_HEARTBEATS.lock().unwrap();
        heartbeats.insert(client_id.to_string(), now);
    }

    if became_online {
        emit_clients_changed(app);
    }

    became_online
}

fn mark_client_connected(app: &AppHandle, client_id: &str, client_name: String) -> bool {
    let became_connected = {
        let mut active_connections = ACTIVE_CONNECTIONS.lock().unwrap();
        active_connections.insert(client_id.to_string())
    };

    if became_connected {
        let event = SessionEvent {
            client_id: client_id.to_string(),
            client_name,
        };
        emit_session_event(app, "device_connected", &event);
        emit_clients_changed(app);
    }

    became_connected
}

fn mark_client_offline(app: &AppHandle, client_id: &str) -> bool {
    let was_online = {
        let mut online_clients = ONLINE_CLIENTS.lock().unwrap();
        online_clients.remove(client_id)
    };

    {
        let mut heartbeats = LAST_HEARTBEATS.lock().unwrap();
        heartbeats.remove(client_id);
    }

    let was_connected = {
        let mut active_connections = ACTIVE_CONNECTIONS.lock().unwrap();
        active_connections.remove(client_id)
    };

    if was_connected {
        let event = SessionEvent {
            client_id: client_id.to_string(),
            client_name: get_client_name(client_id),
        };
        emit_session_event(app, "device_disconnected", &event);
    }

    if was_online || was_connected {
        emit_clients_changed(app);
    }

    was_online || was_connected
}

fn remove_ws_connection(client_id: &str, connection_id: &str) {
    let mut connections = WS_CONNECTIONS.lock().unwrap();
    if matches!(
        connections.get(client_id),
        Some(handle) if handle.connection_id == connection_id
    ) {
        connections.remove(client_id);
    }
}

pub fn cleanup_stale_presence(app: &AppHandle, timeout_ms: u64) {
    let now = current_timestamp_ms();
    let stale_client_ids = {
        let heartbeats = LAST_HEARTBEATS.lock().unwrap();
        heartbeats
            .iter()
            .filter_map(|(client_id, last_seen_at)| {
                if now.saturating_sub(*last_seen_at) > timeout_ms {
                    Some(client_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    for client_id in stale_client_ids {
        let _ = mark_client_offline(app, &client_id);
    }
}

async fn ping_ip_address(ip: &str) -> bool {
    let url = format!("http://{}:3001/ping", ip);
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    match client.get(&url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

async fn auth_pair(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<PairRequestInfo>,
) -> Json<PairResponse> {
    let (tx, rx) = oneshot::channel();

    {
        let mut pending_requests = PENDING_REQUESTS.lock().unwrap();
        pending_requests.insert(payload.client_id.clone(), tx);
    }

    if let Err(error) = state.tauri_app.emit("pair_request", payload.clone()) {
        log::error!("Failed to emit pair request: {}", error);
        let mut pending_requests = PENDING_REQUESTS.lock().unwrap();
        pending_requests.remove(&payload.client_id);

        return Json(PairResponse {
            success: false,
            token: None,
            msg: "Internal server error".into(),
            device_id: None,
            device_name: None,
        });
    }

    match rx.await {
        Ok(true) => {
            let token = Uuid::new_v4().to_string();
            let client = PairedClient {
                client_id: payload.client_id,
                client_name: payload.client_name,
                token_hash: token.clone(),
                last_seen_at: current_timestamp_ms(),
                last_ip: Some(addr.ip().to_string()),
            };

            let (device_id, device_name) = {
                let mut store = GLOBAL_STORE.lock().unwrap();
                store.add_paired_client(client);
                (store.data.device_id.clone(), store.data.device_name.clone())
            };

            emit_clients_changed(&state.tauri_app);

            Json(PairResponse {
                success: true,
                token: Some(token),
                msg: "Pairing successful".into(),
                device_id: Some(device_id),
                device_name: Some(device_name),
            })
        }
        Ok(false) => Json(PairResponse {
            success: false,
            token: None,
            msg: "Pairing was denied by PC user".into(),
            device_id: None,
            device_name: None,
        }),
        Err(_) => Json(PairResponse {
            success: false,
            token: None,
            msg: "Pairing request timed out or cancelled".into(),
            device_id: None,
            device_name: None,
        }),
    }
}

async fn auth_verify(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<VerifyRequest>,
) -> Json<VerifyResponse> {
    let client_name = sync_client_state(
        &payload.client_id,
        &payload.token,
        Some(&addr.ip().to_string()),
        payload.client_name.as_deref(),
    );

    if let Some(client_name) = client_name {
        let _ = mark_client_online(&state.tauri_app, &payload.client_id);
        log::debug!("Verified client {} ({})", client_name, payload.client_id);
        return Json(VerifyResponse {
            success: true,
            msg: "Verified".into(),
        });
    }

    Json(VerifyResponse {
        success: false,
        msg: "Invalid or missing token".into(),
    })
}

async fn ws_session(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<WsSessionQuery>,
) -> impl IntoResponse {
    let client_name = match sync_client_state(
        &query.client_id,
        &query.token,
        Some(&addr.ip().to_string()),
        query.client_name.as_deref(),
    ) {
        Some(client_name) => client_name,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    ws.on_upgrade(move |socket| {
        handle_ws_session(
            state.tauri_app,
            socket,
            query.client_id,
            client_name,
            query.token,
            addr.ip().to_string(),
        )
    })
}

async fn handle_ws_session(
    app: AppHandle,
    socket: WebSocket,
    client_id: String,
    client_name: String,
    token: String,
    ip: String,
) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<WsServerEvent>();

    let previous = {
        let mut connections = WS_CONNECTIONS.lock().unwrap();
        connections.insert(
            client_id.clone(),
            WsConnectionHandle {
                connection_id: connection_id.clone(),
                sender: out_tx.clone(),
            },
        )
    };

    if let Some(previous) = previous {
        let _ = previous.sender.send(WsServerEvent::Disconnect {
            reason: "新的会话已接管当前连接".into(),
        });
    }

    let _ = mark_client_online(&app, &client_id);
    let _ = mark_client_connected(&app, &client_id, client_name.clone());

    let _ = out_tx.send(WsServerEvent::SessionReady {
        client_id: client_id.clone(),
        client_name: client_name.clone(),
    });
    let _ = out_tx.send(WsServerEvent::TasksSync {
        tasks: scheduler::list_tasks(),
    });

    let writer = tokio::spawn(async move {
        while let Some(event) = out_rx.recv().await {
            let payload = match serde_json::to_string(&event) {
                Ok(payload) => payload,
                Err(error) => {
                    log::error!("Failed to serialize ws event: {}", error);
                    continue;
                }
            };

            if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = ws_receiver.next().await {
        let Ok(message) = message else {
            break;
        };

        match message {
            Message::Text(text) => {
                let Ok(event) = serde_json::from_str::<WsClientEvent>(&text) else {
                    continue;
                };

                match event {
                    WsClientEvent::Heartbeat => {
                        let _ =
                            sync_client_state(&client_id, &token, Some(&ip), Some(&client_name));
                        let _ = mark_client_online(&app, &client_id);
                    }
                    WsClientEvent::RequestTasksSync => {
                        let _ = out_tx.send(WsServerEvent::TasksSync {
                            tasks: scheduler::list_tasks(),
                        });
                    }
                    WsClientEvent::Disconnect => break,
                }
            }
            Message::Ping(_) | Message::Pong(_) => {
                let _ = mark_client_online(&app, &client_id);
            }
            Message::Close(_) => break,
            Message::Binary(_) => {}
        }
    }

    writer.abort();
    remove_ws_connection(&client_id, &connection_id);
    let _ = mark_client_offline(&app, &client_id);
}

async fn web_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if let Some(response) = reject_non_lan(&addr) {
        return response.into_response();
    }

    ws.on_upgrade(move |socket| handle_web_ws(state.tauri_app, socket, addr))
        .into_response()
}

async fn handle_web_ws(app: AppHandle, socket: WebSocket, addr: SocketAddr) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<WebServerEvent>();

    WEB_CONNECTIONS
        .lock()
        .unwrap()
        .insert(connection_id.clone(), out_tx.clone());

    let _ = out_tx.send(web_state_event());

    let writer_connection_id = connection_id.clone();
    let writer = tokio::spawn(async move {
        while let Some(event) = out_rx.recv().await {
            let payload = match serde_json::to_string(&event) {
                Ok(payload) => payload,
                Err(error) => {
                    log::error!("Failed to serialize web ws event: {}", error);
                    continue;
                }
            };

            if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }

        WEB_CONNECTIONS
            .lock()
            .unwrap()
            .remove(&writer_connection_id);
    });

    while let Some(message) = ws_receiver.next().await {
        let Ok(message) = message else {
            break;
        };

        match message {
            Message::Text(text) => {
                let Ok(event) = serde_json::from_str::<WebClientEvent>(&text) else {
                    continue;
                };

                match event {
                    WebClientEvent::Heartbeat => {
                        let _ = out_tx.send(WebServerEvent::Pong);
                    }
                    WebClientEvent::RequestStateSync { client_info } => {
                        let _ = web_origin(&addr, client_info.as_ref());
                        let _ = out_tx.send(web_state_event());
                    }
                    WebClientEvent::ExecuteFeature {
                        request_id,
                        client_info,
                        command,
                    } => {
                        let origin = web_origin(&addr, client_info.as_ref());
                        let event = match execute_feature_command_with_origin(
                            &app, command, origin, None,
                        ) {
                            Ok(result) => WebServerEvent::FeatureResult {
                                request_id,
                                success: true,
                                msg: result.message.clone(),
                                result: Some(result),
                            },
                            Err(error) => WebServerEvent::FeatureResult {
                                request_id,
                                success: false,
                                msg: error.to_string(),
                                result: None,
                            },
                        };
                        let _ = out_tx.send(event);
                        let _ = out_tx.send(web_state_event());
                    }
                    WebClientEvent::Disconnect => break,
                }
            }
            Message::Ping(_) | Message::Pong(_) => {}
            Message::Close(_) => break,
            Message::Binary(_) => {}
        }
    }

    log::debug!("Web console websocket {} closed", addr);
    WEB_CONNECTIONS.lock().unwrap().remove(&connection_id);
    writer.abort();
}

pub fn broadcast_web_state_sync() {
    let event = web_state_event();
    let senders = {
        let connections = WEB_CONNECTIONS.lock().unwrap();
        connections.values().cloned().collect::<Vec<_>>()
    };

    for sender in senders {
        let _ = sender.send(event.clone());
    }
}

pub fn broadcast_clipboard_sync() {
    let event = WebServerEvent::ClipboardSync {
        messages: clipboard_sync::list_messages().unwrap_or_default(),
    };
    let senders = {
        let connections = WEB_CONNECTIONS.lock().unwrap();
        connections.values().cloned().collect::<Vec<_>>()
    };

    for sender in senders {
        let _ = sender.send(event.clone());
    }
}

pub fn broadcast_tasks_sync() {
    let tasks = scheduler::list_tasks();
    let senders = {
        let connections = WS_CONNECTIONS.lock().unwrap();
        connections
            .values()
            .map(|handle| handle.sender.clone())
            .collect::<Vec<_>>()
    };

    for sender in senders {
        let _ = sender.send(WsServerEvent::TasksSync {
            tasks: tasks.clone(),
        });
    }
}

pub fn disconnect_client_session(client_id: &str, reason: &str) {
    let sender = {
        let connections = WS_CONNECTIONS.lock().unwrap();
        connections
            .get(client_id)
            .map(|handle| handle.sender.clone())
    };

    if let Some(sender) = sender {
        let _ = sender.send(WsServerEvent::Disconnect {
            reason: reason.to_string(),
        });
    }
}

async fn auth_connect(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<SessionRequest>,
) -> Json<serde_json::Value> {
    match sync_client_state(
        &payload.client_id,
        &payload.token,
        Some(&addr.ip().to_string()),
        payload.client_name.as_deref(),
    ) {
        Some(_) => {
            let _ = mark_client_online(&state.tauri_app, &payload.client_id);
            Json(serde_json::json!({ "success": true }))
        }
        None => Json(serde_json::json!({ "success": false, "msg": "Unauthorized" })),
    }
}

async fn auth_disconnect(
    State(state): State<AppState>,
    Json(payload): Json<SessionRequest>,
) -> Json<serde_json::Value> {
    let authorized = sync_client_state(
        &payload.client_id,
        &payload.token,
        None,
        payload.client_name.as_deref(),
    )
    .is_some();

    if !authorized {
        return Json(serde_json::json!({ "success": false, "msg": "Unauthorized" }));
    }

    disconnect_client_session(&payload.client_id, "移动端主动断开");
    let _ = mark_client_offline(&state.tauri_app, &payload.client_id);
    Json(serde_json::json!({ "success": true }))
}

async fn auth_heartbeat(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<SessionRequest>,
) -> Json<serde_json::Value> {
    let client_name = match sync_client_state(
        &payload.client_id,
        &payload.token,
        Some(&addr.ip().to_string()),
        payload.client_name.as_deref(),
    ) {
        Some(client_name) => client_name,
        None => {
            return Json(serde_json::json!({ "success": false, "msg": "Unauthorized" }));
        }
    };

    let _ = mark_client_online(&state.tauri_app, &payload.client_id);
    log::debug!(
        "Heartbeat received from {} ({})",
        client_name,
        payload.client_id
    );

    Json(serde_json::json!({ "success": true }))
}

async fn features_catalog(
    Json(payload): Json<MobileFeatureCatalogRequest>,
) -> Json<FeatureCatalogResponse> {
    if !is_client_authorized(&payload.client_id, &payload.token) {
        return Json(FeatureCatalogResponse {
            success: false,
            msg: "Unauthorized".into(),
            groups: vec![],
            snapshot: None,
        });
    }

    match get_feature_snapshot() {
        Ok(snapshot) => Json(FeatureCatalogResponse {
            success: true,
            msg: "OK".into(),
            groups: get_feature_groups(),
            snapshot: Some(snapshot),
        }),
        Err(error) => Json(FeatureCatalogResponse {
            success: false,
            msg: error.to_string(),
            groups: vec![],
            snapshot: None,
        }),
    }
}

async fn features_execute(
    State(state): State<AppState>,
    Json(payload): Json<MobileFeatureExecuteRequest>,
) -> Json<FeatureExecuteResponse> {
    if !is_client_authorized(&payload.client_id, &payload.token) {
        return Json(FeatureExecuteResponse {
            success: false,
            msg: "Unauthorized".into(),
            result: None,
        });
    }

    let origin = TaskOrigin::mobile(
        payload.client_id.clone(),
        get_client_name(&payload.client_id),
    );

    match execute_feature_command_with_origin(&state.tauri_app, payload.command, origin, None) {
        Ok(result) => Json(FeatureExecuteResponse {
            success: true,
            msg: result.message.clone(),
            result: Some(result),
        }),
        Err(error) => Json(FeatureExecuteResponse {
            success: false,
            msg: error.to_string(),
            result: None,
        }),
    }
}

async fn tasks_list(Json(payload): Json<MobileTaskListRequest>) -> Json<TaskListResponse> {
    if !is_client_authorized(&payload.client_id, &payload.token) {
        return Json(TaskListResponse {
            success: false,
            msg: "Unauthorized".into(),
            tasks: vec![],
        });
    }

    Json(TaskListResponse {
        success: true,
        msg: "OK".into(),
        tasks: scheduler::list_tasks(),
    })
}

async fn tasks_create(
    State(state): State<AppState>,
    Json(payload): Json<MobileTaskCreateRequest>,
) -> Json<TaskCreateResponse> {
    if !is_client_authorized(&payload.client_id, &payload.token) {
        return Json(TaskCreateResponse {
            success: false,
            msg: "Unauthorized".into(),
            task: None,
        });
    }

    if payload.execute_at_ms <= current_timestamp_ms() {
        return Json(TaskCreateResponse {
            success: false,
            msg: "执行时间必须晚于当前时间".into(),
            task: None,
        });
    }

    let task = ScheduledTask::new(
        payload.command,
        payload.execute_at_ms,
        TaskOrigin::mobile(
            payload.client_id.clone(),
            get_client_name(&payload.client_id),
        ),
    );
    let created_task = scheduler::create_task(&state.tauri_app, task);

    Json(TaskCreateResponse {
        success: true,
        msg: "定时任务已创建".into(),
        task: Some(created_task),
    })
}

async fn tasks_cancel(
    State(state): State<AppState>,
    Json(payload): Json<MobileTaskCancelRequest>,
) -> Json<TaskCreateResponse> {
    if !is_client_authorized(&payload.client_id, &payload.token) {
        return Json(TaskCreateResponse {
            success: false,
            msg: "Unauthorized".into(),
            task: None,
        });
    }

    let cancelled_task = scheduler::cancel_task(&state.tauri_app, &payload.task_id);
    match cancelled_task {
        Some(task) => Json(TaskCreateResponse {
            success: true,
            msg: "定时任务已停止".into(),
            task: Some(task),
        }),
        None => Json(TaskCreateResponse {
            success: false,
            msg: "未找到可停止的定时任务".into(),
            task: None,
        }),
    }
}

async fn web_index(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    if let Some(response) = reject_non_lan(&addr) {
        return response.into_response();
    }

    Html(web_console::INDEX_HTML).into_response()
}

async fn web_css(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    if let Some(response) = reject_non_lan(&addr) {
        return response.into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "text/css; charset=utf-8".parse().unwrap(),
    );
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    (headers, web_console::APP_CSS).into_response()
}

fn web_js_response(addr: &SocketAddr, content: &'static str) -> axum::response::Response {
    if let Some(response) = reject_non_lan(addr) {
        return response.into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "text/javascript; charset=utf-8".parse().unwrap(),
    );
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    (headers, content).into_response()
}

async fn web_main_js(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> impl IntoResponse {
    web_js_response(&addr, web_console::MAIN_JS)
}

async fn read_multipart_sync_message(
    mut multipart: Multipart,
) -> Result<(Option<String>, Vec<IncomingAttachment>, ClipboardSyncSourceKind, Option<WebClientInfo>), String> {
    let mut text = None;
    let mut attachments = Vec::new();
    let mut source_kind = ClipboardSyncSourceKind::Web;
    let mut client_info = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| format!("读取上传内容失败: {error}"))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "text" => {
                text = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| format!("读取文本失败: {error}"))?,
                );
            }
            "source_kind" => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("读取来源失败: {error}"))?;
                if value.trim().eq_ignore_ascii_case("pc") {
                    source_kind = ClipboardSyncSourceKind::Pc;
                }
            }
            "client_info" => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("读取设备信息失败: {error}"))?;
                client_info = serde_json::from_str::<WebClientInfo>(&value).ok();
            }
            "files" | "files[]" => {
                let Some(file_name) = field.file_name().map(ToString::to_string) else {
                    continue;
                };
                let mime_type = field.content_type().map(ToString::to_string);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| format!("读取文件失败: {error}"))?;
                attachments.push(IncomingAttachment {
                    file_name,
                    mime_type,
                    bytes: bytes.to_vec(),
                });
            }
            _ => {}
        }
    }

    Ok((text, attachments, source_kind, client_info))
}

async fn web_clipboard_sync_history(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !is_lan_or_loopback(&addr) {
        return sync_error_response(
            StatusCode::FORBIDDEN,
            "Web console is only available on the local network",
        );
    }

    match clipboard_sync::list_messages() {
        Ok(messages) => Json(ClipboardSyncResponse {
            success: true,
            msg: "OK".into(),
            message: None,
            messages,
        })
        .into_response(),
        Err(error) => sync_error_response(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn web_clipboard_sync_create(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    multipart: Multipart,
) -> impl IntoResponse {
    if !is_lan_or_loopback(&addr) {
        return sync_error_response(
            StatusCode::FORBIDDEN,
            "Web console is only available on the local network",
        );
    }

    let (text, attachments, source_kind, client_info) = match read_multipart_sync_message(multipart).await {
        Ok(value) => value,
        Err(error) => return sync_error_response(StatusCode::BAD_REQUEST, error),
    };
    let write_clipboard = matches!(source_kind, ClipboardSyncSourceKind::Web);
    let source = clipboard_source(&addr, source_kind, client_info.as_ref());

    match clipboard_sync::create_message(text, attachments, source, write_clipboard).await {
        Ok(message) => {
            emit_clipboard_sync_changed(&state.tauri_app);
            broadcast_clipboard_sync();
            Json(ClipboardSyncResponse {
                success: true,
                msg: "已同步".into(),
                message: Some(message),
                messages: clipboard_sync::list_messages().unwrap_or_default(),
            })
            .into_response()
        }
        Err(error) => sync_error_response(StatusCode::BAD_REQUEST, error),
    }
}

async fn web_clipboard_sync_file(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path((message_id, attachment_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Some(response) = reject_non_lan(&addr) {
        return response.into_response();
    }

    let (path, attachment) = match clipboard_sync::attachment_path(&message_id, &attachment_id) {
        Ok(value) => value,
        Err(error) => return (StatusCode::NOT_FOUND, error).into_response(),
    };

    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) => return (StatusCode::NOT_FOUND, error.to_string()).into_response(),
    };

    let mut headers = HeaderMap::new();
    if let Some(mime_type) = attachment.mime_type.as_deref() {
        if let Ok(value) = mime_type.parse() {
            headers.insert(header::CONTENT_TYPE, value);
        }
    }
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    let ascii_file_name = attachment
        .file_name
        .chars()
        .filter(|ch| ch.is_ascii() && !matches!(ch, '"' | '\\' | '\r' | '\n'))
        .collect::<String>();
    let disposition = if ascii_file_name.is_empty() {
        "attachment".to_string()
    } else {
        format!("attachment; filename=\"{}\"", ascii_file_name)
    };
    if let Ok(value) = disposition.parse() {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }

    (headers, bytes).into_response()
}

async fn web_state(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Json<WebStateResponse> {
    if !is_lan_or_loopback(&addr) {
        return Json(WebStateResponse {
            success: false,
            msg: "Web console is only available on the local network".into(),
            groups: vec![],
            snapshot: None,
            tasks: vec![],
            history: vec![],
            sync_messages: vec![],
        });
    }

    Json(web_state_payload())
}

async fn web_features_execute(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<WebFeatureExecuteRequest>,
) -> Json<FeatureExecuteResponse> {
    if !is_lan_or_loopback(&addr) {
        return Json(FeatureExecuteResponse {
            success: false,
            msg: "Web console is only available on the local network".into(),
            result: None,
        });
    }

    let origin = web_origin(&addr, payload.client_info.as_ref());
    match execute_feature_command_with_origin(&state.tauri_app, payload.command, origin, None) {
        Ok(result) => {
            broadcast_web_state_sync();
            Json(FeatureExecuteResponse {
                success: true,
                msg: result.message.clone(),
                result: Some(result),
            })
        }
        Err(error) => {
            broadcast_web_state_sync();
            Json(FeatureExecuteResponse {
                success: false,
                msg: error.to_string(),
                result: None,
            })
        }
    }
}

async fn web_tasks_create(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<WebTaskCreateRequest>,
) -> Json<TaskCreateResponse> {
    if !is_lan_or_loopback(&addr) {
        return Json(TaskCreateResponse {
            success: false,
            msg: "Web console is only available on the local network".into(),
            task: None,
        });
    }

    if payload.execute_at_ms <= current_timestamp_ms() {
        return Json(TaskCreateResponse {
            success: false,
            msg: "执行时间必须晚于当前时间".into(),
            task: None,
        });
    }

    let origin = web_origin(&addr, payload.client_info.as_ref());
    let task = ScheduledTask::new(payload.command, payload.execute_at_ms, origin.clone());
    emit_web_notice(&state.tauri_app, &origin, format!("发起了{}", task.title));
    let created_task = scheduler::create_task(&state.tauri_app, task);
    broadcast_web_state_sync();

    Json(TaskCreateResponse {
        success: true,
        msg: "定时任务已创建".into(),
        task: Some(created_task),
    })
}

async fn web_tasks_cancel(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<WebTaskCancelRequest>,
) -> Json<TaskCreateResponse> {
    if !is_lan_or_loopback(&addr) {
        return Json(TaskCreateResponse {
            success: false,
            msg: "Web console is only available on the local network".into(),
            task: None,
        });
    }

    let origin = web_origin(&addr, payload.client_info.as_ref());
    let cancelled_task = scheduler::cancel_task(&state.tauri_app, &payload.task_id);
    match cancelled_task {
        Some(task) => {
            emit_web_notice(&state.tauri_app, &origin, format!("停止了{}", task.title));
            broadcast_web_state_sync();
            Json(TaskCreateResponse {
                success: true,
                msg: "定时任务已停止".into(),
                task: Some(task),
            })
        }
        None => Json(TaskCreateResponse {
            success: false,
            msg: "未找到可停止的定时任务".into(),
            task: None,
        }),
    }
}

#[tauri::command]
pub fn get_clients_with_status() -> Vec<ClientInfo> {
    let snapshots = collect_client_snapshots();
    let online_clients = ONLINE_CLIENTS.lock().unwrap();
    let active_connections = ACTIVE_CONNECTIONS.lock().unwrap();

    snapshots
        .into_iter()
        .map(|snapshot| {
            let is_online = online_clients.contains(&snapshot.client_id);
            ClientInfo {
                client_id: snapshot.client_id.clone(),
                client_name: snapshot.client_name,
                last_seen_at: snapshot.last_seen_at,
                last_ip: snapshot.last_ip,
                is_online,
                is_connected: is_online && active_connections.contains(&snapshot.client_id),
            }
        })
        .collect()
}

#[tauri::command]
pub async fn ping_mobile_device(ip: String) -> Result<bool, String> {
    Ok(ping_ip_address(&ip).await)
}

#[tauri::command]
pub async fn notify_mobile_disconnect(ip: String) -> Result<(), String> {
    let url = format!("http://{}:3001/disconnect", ip);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1000))
        .build()
        .map_err(|error| error.to_string())?;

    let _ = client.post(&url).send().await;
    Ok(())
}
