use axum::{Router, routing::get};
use tokio::net::TcpListener;

async fn health() -> &'static str {
    "hello from RustForge API backend!"
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health));

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("RustForge API is up and running at http://127.0.0.1:3000");

    axum::serve(listener, app).await.expect("Server failed");
}
