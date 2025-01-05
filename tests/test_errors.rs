use floz::errors::{ApiError, ErrorCode};

#[test]
fn test_api_error_constructors() {
    let err = ApiError::new(ErrorCode::GenericError, "test error");
    assert_eq!(err.code, ErrorCode::GenericError);
    assert_eq!(err.message, "test error");

    let err = ApiError::bad_request("bad");
    assert_eq!(err.code, ErrorCode::BadRequest);
    assert_eq!(err.message, "bad");

    let err = ApiError::not_found("missing");
    assert_eq!(err.code, ErrorCode::NotFound);
    assert_eq!(err.message, "missing");

    let err = ApiError::forbidden("stop");
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert_eq!(err.message, "stop");

    let err = ApiError::internal("broken");
    assert_eq!(err.code, ErrorCode::InternalServerError);
    assert_eq!(err.message, "broken");
}

#[test]
fn test_api_error_properties() {
    let err = ApiError::new(ErrorCode::NotFound, "not found here");
    assert_eq!(err.code(), &ErrorCode::NotFound);
    assert_eq!(err.message(), "not found here");
    assert_eq!(err.to_string(), "NotFound: not found here");
}

#[test]
fn test_api_error_from_string() {
    let s = String::from("string error");
    let err: ApiError = s.into();
    assert_eq!(err.code, ErrorCode::GenericError);
    assert_eq!(err.message, "string error");

    let str_ref = "str diff";
    let err: ApiError = str_ref.into();
    assert_eq!(err.code, ErrorCode::GenericError);
    assert_eq!(err.message, "str diff");
}

#[test]
fn test_api_error_from_uuid() {
    let res = uuid::Uuid::parse_str("invalid_uuid");
    assert!(res.is_err());
    let err: ApiError = res.unwrap_err().into();
    assert_eq!(err.code, ErrorCode::InvalidUUID);
}

#[test]
fn test_api_error_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("some error");
    let err: ApiError = anyhow_err.into();
    assert_eq!(err.code, ErrorCode::GenericError);
    assert_eq!(err.message, "some error");
}
