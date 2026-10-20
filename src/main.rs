use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex};

#[derive(Clone, Serialize)]
struct Task {
    id: u64,
    title: String,
}

#[derive(Deserialize)]
struct CreateTask {
    title: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

type SharedTasks = Arc<Mutex<Vec<Task>>>;

async fn create_task(
    State(tasks): State<SharedTasks>,
    Json(payload): Json<CreateTask>,
) -> Result<(StatusCode, Json<Task>), (StatusCode, Json<ErrorResponse>)> {
    let title = payload.title.trim();

    if title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Task title cannot be empty".to_string(),
            }),
        ));
    }

    let mut tasks = tasks.lock().await;

    let id = tasks.last().map_or(1, |task| task.id + 1);

    let task = Task {
        id,
        title: title.to_string(),
    };

    tasks.push(task.clone());

    Ok((StatusCode::CREATED, Json(task)))
}

async fn get_task(
    Path(id): Path<u64>,
    State(tasks): State<SharedTasks>,
) -> Result<Json<Task>, StatusCode> {
    let task = tasks
        .lock()
        .await
        .iter()
        .find(|task| task.id == id)
        .cloned();

    match task {
        Some(task) => Ok(Json(task)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "RustForge API"
    }))
}

#[tokio::main]
async fn main() {
    let tasks: SharedTasks = Arc::new(Mutex::new(Vec::new()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/tasks", post(create_task))
        .route("/tasks/{id}", get(get_task))
        .with_state(tasks);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("RustForge API is up and running at http://127.0.0.1:3000");

    axum::serve(listener, app).await.expect("Server failed");
}
