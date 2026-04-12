use floz::prelude::*;
use serde::{Deserialize, Serialize};
use floz::utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct User {
    #[schema(example = "1")]
    pub id: i32,
    #[schema(example = "Alice")]
    pub name: String,
    #[schema(example = "alice@example.com")]
    pub email: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[route(
    get: "/users/:id",
    tag: "Users",
    resps: [
        (200, "User found", Json<User>),
        (404, "User not found", Json<ErrorResponse>)
    ]
)]
async fn get_user(
    path: Path<i32>,
) -> Resp {
    let id = path.into_inner();
    
    if id == 1 {
        let user = User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        Resp::Ok().json(&user)
    } else {
        Resp::NotFound().json(&ErrorResponse {
            message: "User not found".to_string(),
        })
    }
}

#[route(
    get: "/protected",
    tag: "Protected",
    resps: [(200, "Success")],
    middleware: [floz::web::middleware::Logger::default()]
)]
async fn protected_route() -> Resp {
    Resp::Ok().body("Accessed protected route")
}

#[floz::main]
async fn main() -> std::io::Result<()> {
    App::new()
        .server(ServerConfig::new().with_default_port(8080))
        .run()
        .await
}
