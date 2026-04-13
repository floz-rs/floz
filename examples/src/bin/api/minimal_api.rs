use floz::prelude::*;

#[route(
    get: "/users/:id",
    tag: "Users",
    desc: "Get user by ID",
    resps: [
        (200, "User found"),
        (404, "User not found"),
    ],
)]
async fn get_user(path: Path<i32>) -> Resp {
    let id = path.into_inner();
    Resp::Ok().json(&json!({
        "id": id,
        "name": "Alice"
    }))
}

#[route(
    get: "/health",
    tag: "System",
    desc: "Health check",
    resps: [(200, "Ready")]
)]
async fn health() -> Resp {
    Resp::Ok().body("OK")
}

#[floz::main]
async fn main() -> std::io::Result<()> {
    App::new().run().await
}
