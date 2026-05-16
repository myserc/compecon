use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Router,
    Json,
};
use std::sync::{Arc, Mutex};
use crate::engine::MacroStats;
use crossbeam::channel::Sender;

pub enum ControlCommand {
    EconomicShock(f64),
    DeficitSpending(u64),
}

pub struct DashboardState {
    pub last_stats: Mutex<MacroStats>,
    pub control_tx: Sender<ControlCommand>,
}

pub async fn start_dashboard(state: Arc<DashboardState>) {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/control/shock", post(shock_handler))
        .route("/api/control/deficit_spending", post(deficit_spending_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    state: axum::extract::State<Arc<DashboardState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn shock_handler(
    state: axum::extract::State<Arc<DashboardState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let intensity = payload["intensity"].as_f64().unwrap_or(0.0);
    state.control_tx.send(ControlCommand::EconomicShock(intensity)).unwrap();
    axum::http::StatusCode::OK
}

async fn deficit_spending_handler(
    state: axum::extract::State<Arc<DashboardState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let amount = payload["amount"].as_u64().unwrap_or(0);
    state.control_tx.send(ControlCommand::DeficitSpending(amount)).unwrap();
    axum::http::StatusCode::OK
}

async fn handle_socket(mut socket: WebSocket, state: axum::extract::State<Arc<DashboardState>>) {
    loop {
        let stats = {
            let s = state.last_stats.lock().unwrap();
            s.clone()
        };

        let msg = serde_json::to_string(&stats).unwrap();
        if socket.send(Message::Text(msg)).await.is_err() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
}
