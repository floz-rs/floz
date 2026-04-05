use floz::ntex::web::types::Json;
use floz::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 1. Define Background Tasks
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[task(queue = "emails", retries = 3)]
async fn send_welcome_email(user_name: String) -> Result<(), floz::worker::TaskError> {
    println!(">>> [WORKER] Starting to send welcome email to {}...", user_name);
    
    // Simulate some work, e.g., network request
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    println!(">>> [WORKER] ✓ Welcome email successfully sent to {}!", user_name);
    Ok(())
}

#[task(queue = "default")]
async fn cleanup_inactive_accounts(limit: i32) -> Result<(), floz::worker::TaskError> {
    println!(">>> [WORKER] Running cleanup job (limit: {})", limit);
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!(">>> [WORKER] ✓ Cleanup finished.");
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 2. Define HTTP Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Serialize, Deserialize, Default)]
struct UserReq {
    name: String,
}

#[route(
    post: "/register",
    tag: "Users",
    desc: "Register a new user and dispatch a background email task",
    resps: [(200, "User registered successfully")]
)]
async fn register_user(state: floz::ntex::web::types::State<AppContext>, req: Json<UserReq>) -> Result<floz::ntex::web::HttpResponse, floz::ntex::web::Error> {
    println!("API: Received registration for user '{}'", req.name);
    
    // Fire and forget task execution
    send_welcome_email.dispatch(&state, req.name.clone()).await.map_err(|e| {
        println!("Failed to dispatch task: {e}");
        floz::ntex::web::error::ErrorInternalServerError("Queue dispatch error")
    })?;

    // Schedule a cleanup job 10 seconds into the future
    cleanup_inactive_accounts
        .delay(Duration::from_secs(10))
        .dispatch(&state, 100)
        .await
        .map_err(|e| {
            floz::ntex::web::error::ErrorInternalServerError(format!("Dispatch error: {}", e))
        })?;

    Ok(floz::ntex::web::HttpResponse::Ok().json(&serde_json::json!({
        "status": "registered",
        "user": req.name,
        "message": "Welcome email has been queued and will be sent shortly."
    })))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 3. Start the Server and Workers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[floz::main]
async fn main() -> std::io::Result<()> {
    // A redis instance is required! You can use docker:
    // docker run -p 6379:6379 -d redis
    std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
    
    // Ensure you use the App builder with `.with_worker()`
    App::new()
        .with_worker(2) // Start 2 concurrent worker threads fetching from queues
        .server(
            ServerConfig::new()
                .with_default_port(8080)
        )
        .run()
        .await
}
