use floz::middleware::cors::Cors;
use ntex::http::header;

#[ntex::test]
async fn test_cors_builder() {
    let _cors = Cors::new()
        .allow_origin("https://example.com")
        .allow_methods(["GET", "POST"])
        .allow_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
        .expose_headers(vec![header::CONTENT_LENGTH])
        .supports_credentials()
        .max_age(7200);
}

#[ntex::test]
async fn test_cors_permissive() {
    let _cors = Cors::permissive();
}

#[ntex::test]
async fn test_cors_integration_preflight() {
    use ntex::web::{self, App, HttpResponse};
    use ntex::web::test::{init_service, TestRequest};

    let app = init_service(
        App::new()
            .middleware(floz::middleware::FlozPipeline::new(
                floz::middleware::Stack {
                    inner: floz::middleware::EmptyStack,
                    outer: Cors::permissive()
                }
            ))
            .route("/", web::get().to(|| async { HttpResponse::Ok() }))
    ).await;

    let req = TestRequest::with_uri("/")
        .method(ntex::http::Method::OPTIONS)
        .header(header::ORIGIN, "https://local.dev")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .to_request();

    let resp = ntex::web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let allowed_origin = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap();
    assert_eq!(allowed_origin, "https://local.dev");
}

#[ntex::test]
async fn test_cors_integration_request() {
    use ntex::web::{self, App, HttpResponse};
    use ntex::web::test::{init_service, TestRequest};

    let app = init_service(
        App::new()
            .middleware(floz::middleware::FlozPipeline::new(
                floz::middleware::Stack {
                    inner: floz::middleware::EmptyStack,
                    outer: Cors::permissive()
                }
            ))
            .route("/", web::get().to(|| async { HttpResponse::Ok() }))
    ).await;

    let req = TestRequest::get()
        .uri("/")
        .header(header::ORIGIN, "https://local.dev")
        .to_request();

    let resp = ntex::web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let allowed_origin = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap();
    assert_eq!(allowed_origin, "https://local.dev");
}
