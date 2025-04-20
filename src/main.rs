use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;

#[derive(Deserialize, Clone)]
enum AppState {
    Tasks(String),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer());

    let (task_sender, task_recv) = mpsc::channel::<AppState>(10);
    tokio::spawn(background(task_recv));
    let v1_api = Router::new()
        .route("/status", get(status))
        .route("/stop/{block_case}", post(stop))
        .route("/testcases", get(testcases));
    let task_sender = Arc::new(task_sender);
    let app = Router::new()
        .route("/", get(|| async { "hello" }))
        .nest("/v1", v1_api)
        .with_state(task_sender)
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:40000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn status() -> Response {
    Json(json!({"status":"ok"})).into_response()
}

async fn stop(Path(block_case): Path<i64>) -> Response {
    Json(json!({"block_case":block_case})).into_response()
}

async fn testcases(State(task_sender): State<Arc<Sender<AppState>>>, body: Bytes) -> Response {
    // 解析请求体为任务结构
    let task = match serde_json::from_slice::<AppState>(&body) {
        Ok(task) => task,
        Err(e) => {
            return Json(json!({
                "status": "error",
                "message": format!("Failed to parse task: {}", e)
            }))
            .into_response();
        }
    };
    match task_sender.send(task).await {
        Ok(_) => Json(json!({
            "status": "success",
            "message": "Task added to queue",
        }))
        .into_response(),
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("Failed to add task to queue: {}", e)
        }))
        .into_response(),
    }
}

async fn background(mut task_recv: tokio::sync::mpsc::Receiver<AppState>) {
    while let Some(task) = task_recv.recv().await {
        match task {
            AppState::Tasks(task) => {
                println!("task: {}", task);
            }
        }
    }
}
