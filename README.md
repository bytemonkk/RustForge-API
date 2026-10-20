# RustForge API

A production-style REST API built with Rust, Axum, SQLx, and PostgreSQL.

## Learning Goals

- Learn Rust through practical backend development.
- Build REST endpoints using Axum.
- Integrate PostgreSQL using SQLx.
- Implement authentication and CRUD operations.
- Write unit and integration tests.

## Progress

- [x] Initialize Rust project
- [x] Verify compilation and test harness
- [x] Build the first HTTP endpoint
- [x] Return JSON health responses
- [x] Implement in-memory task creation and retrieval
- [x] Handle missing tasks with HTTP 404
- [ ] Integrate PostgreSQL with Docker
- [ ] Implement CRUD and authentication
- [ ] Add middleware and integration tests

# dependencies dobby!
- Axum — defines routes and handles HTTP requests.
- Tokio — runs our asynchronous server.
- Serde is a Rust library for serializing and deserializing data. We'll use it to convert a Rust struct into JSON.