mod api; // <--- Import the new module we just made

use axum::{
    routing::{get, post},
    Json, Router,
};
use tokio::net::TcpListener;
use tracing::info;
use crate::api::{RunRequest, RunResponse};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Define the Routes
    // We added ".route("/run", post(submit_run))"
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/run", post(submit_run));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Gateway starting on port 3000...");
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "IronClaw Gateway: Operational"
}

// The New Handler
// Axum automatically extracts JSON into the 'payload' variable.
// If the JSON is invalid, Axum rejects it before this function even runs.
async fn submit_run(Json(payload): Json<RunRequest>) -> Json<RunResponse> {
    info!("Received task from tenant: {}", payload.tenant_id);
    info!("Task: {}", payload.task);

    // TODO: Phase 2 - Pass this to the Orchestrator
    
    // Return a Mock Response for now
    Json(RunResponse {
        job_id: "mock-123-xyz".to_string(),
        status: "accepted".to_string(),
    })
}