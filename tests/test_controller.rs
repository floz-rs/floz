use floz::controller::format::JsonResponse;
use floz::controller::pagination::PaginationParams;
use serde::Serialize;
use ntex::http::StatusCode;

#[derive(Serialize)]
struct DummyData {
    name: String,
}

#[test]
fn test_json_response_ok() {
    let data = DummyData { name: "test".to_string() };
    let resp = JsonResponse::ok(&data);
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn test_json_response_with_status() {
    let data = DummyData { name: "test".to_string() };
    let resp = JsonResponse::with_status(&data, 202);
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[test]
fn test_json_response_created() {
    let data = DummyData { name: "test".to_string() };
    let resp = JsonResponse::created(&data);
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[test]
fn test_json_response_no_content() {
    let resp = JsonResponse::no_content();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[test]
fn test_json_response_errors() {
    let resp = JsonResponse::bad_request("bad");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = JsonResponse::not_found("not found");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = JsonResponse::error("server error");
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_pagination_params_default() {
    let params = PaginationParams::default();
    assert_eq!(params.limit, 10);
    assert_eq!(params.offset, 0);
    assert_eq!(params.order_by, "created");
    assert_eq!(params.filter, "");
    assert_eq!(params.search, "");
}

struct DummyModel;

#[test]
fn test_pagination_params_table_name() {
    let table = PaginationParams::table_name::<DummyModel>();
    assert_eq!(table, "DummyModel");
}

#[test]
fn test_pagination_params_for_model() {
    let params = PaginationParams::for_model::<DummyModel>("123".to_string(), "dummy_module");
    assert_eq!(params.table, "DummyModel");
    assert_eq!(params.module_name, "dummy_module");
    assert_eq!(params.id, "123");
    assert_eq!(params.limit, 10);
}

#[test]
fn test_pagination_params_with_model() {
    let base = PaginationParams {
        limit: 50,
        offset: 10,
        order_by: "id".to_string(),
        filter: "active:true".to_string(),
        search: "query".to_string(),
        ..Default::default()
    };
    
    let model_params = base.with_model::<DummyModel>("dummy_module");
    assert_eq!(model_params.limit, 50);
    assert_eq!(model_params.offset, 10);
    assert_eq!(model_params.order_by, "id");
    assert_eq!(model_params.filter, "active:true");
    assert_eq!(model_params.search, "query");
    assert_eq!(model_params.table, "DummyModel");
    assert_eq!(model_params.module_name, "dummy_module");
}
