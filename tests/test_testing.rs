use floz::testing::TestApp;
use floz_macros::route;
use ntex::web::{self, HttpResponse};

// Test dummy route
#[route(get: "/ping", tag: "Test", desc: "Ping pong")]
async fn ping() -> HttpResponse {
    HttpResponse::Ok().body("pong")
}

#[route(post: "/echo", tag: "Test", desc: "Echo back")]
async fn echo(body: web::types::Json<serde_json::Value>) -> HttpResponse {
    HttpResponse::Ok().json(&body.into_inner())
}

#[ntex::test]
async fn test_floz_testing_app() {
    let app = TestApp::new().await;

    // 1. Test GET request
    let resp = app.get("/ping").send().await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text(), "pong");

    // 2. Test POST request with JSON
    let payload = serde_json::json!({"message": "hello test builder"});
    let resp = app
        .post("/echo")
        .json(&payload)
        .header("X-Test-Echo", "true")
        .send()
        .await;

    assert_eq!(resp.status(), 200);
    let echoed_json = resp.json_value();
    assert_eq!(echoed_json, payload);

    println!("Floz test utilities are working correctly!");
}
