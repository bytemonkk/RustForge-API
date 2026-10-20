use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;

#[derive(Clone, Serialize, FromRow)]
struct Task {
    id: i64,
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

async fn create_task(
    State(pool): State<PgPool>,
    payload: Result<Json<CreateTask>, JsonRejection>,
) -> Result<(StatusCode, Json<Task>), (StatusCode, Json<ErrorResponse>)> {
    let Json(payload) = payload.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: err.body_text(),
            }),
        )
    })?;

    let title = payload.title.trim();

    if title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Task title cannot be empty".to_string(),
            }),
        ));
    }

    let task =
        sqlx::query_as::<_, Task>("INSERT INTO tasks (title) VALUES ($1) RETURNING id, title")
            .bind(title)
            .fetch_one(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to create task".to_string(),
                    }),
                )
            })?;

    Ok((StatusCode::CREATED, Json(task)))
}

async fn get_task(
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    let task = sqlx::query_as::<_, Task>("SELECT id, title FROM tasks WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to retrieve task".to_string(),
                }),
            )
        })?;

    match task {
        Some(task) => Ok(Json(task)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Task not found".to_string(),
            }),
        )),
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "RustForge API"
    }))
}

fn build_app(pool: PgPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tasks", post(create_task))
        .route("/tasks/{id}", get(get_task))
        .with_state(pool)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let app = build_app(pool);

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("RustForge API is up and running at http://127.0.0.1:3000");

    axum::serve(listener, app).await.expect("Server failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    async fn test_app() -> Router {
        dotenvy::dotenv().expect("Failed to load .env file");

        let database_url =
            std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run test database migrations");

        build_app(pool)
    }

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        serde_json::from_slice(&body).expect("Response was not valid JSON")
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn create_task_returns_created() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Automated test task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response_json(response).await;
        assert_eq!(body["title"], "Automated test task");
        assert!(body["id"].as_i64().is_some());
    }

    #[tokio::test]
    async fn get_existing_task_returns_ok() {
        let app = test_app().await;

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"Task to retrieve"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::CREATED);

        let created = response_json(create_response).await;
        let id = created["id"].as_i64().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/tasks/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;
        assert_eq!(body["id"], id);
        assert_eq!(body["title"], "Task to retrieve");
    }

    #[tokio::test]
    async fn missing_task_returns_not_found() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/tasks/-999999999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn empty_title_returns_bad_request() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"title":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response_json(response).await;
        assert_eq!(body["error"], "Task title cannot be empty");
    }
}
