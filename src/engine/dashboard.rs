use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};
use crate::engine::MacroStats;

pub struct DashboardState {
    pub last_stats: Mutex<MacroStats>,
}

pub async fn start_dashboard(state: Arc<DashboardState>) {
    let app = Router::new()
        .route("/ws", get(ws_handler))
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
