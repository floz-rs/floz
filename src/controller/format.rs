//! Response formatting utilities.

use ntex::web::HttpResponse;
use serde::Serialize;

/// A convenience wrapper for JSON responses.
pub struct JsonResponse;

impl JsonResponse {
    /// Return a 200 OK JSON response.
    pub fn ok<T: Serialize>(data: &T) -> HttpResponse {
        let body = match std::env::var("SERVER_ENV") {
            Ok(env) if env == "DEV" => serde_json::to_string_pretty(data),
            _ => serde_json::to_string(data),
        };

        HttpResponse::Ok()
            .content_type("application/json")
            .body(body.unwrap_or_default())
    }

    /// Return a JSON response with a custom status code.
    pub fn with_status<T: Serialize>(data: &T, status: u16) -> HttpResponse {
        let body = match std::env::var("SERVER_ENV") {
            Ok(env) if env == "DEV" => serde_json::to_string_pretty(data),
            _ => serde_json::to_string(data),
        };

        HttpResponse::build(
            ntex::http::StatusCode::from_u16(status).unwrap_or(ntex::http::StatusCode::OK),
        )
        .content_type("application/json")
        .body(body.unwrap_or_default())
    }

    /// Return a 201 Created JSON response.
    pub fn created<T: Serialize>(data: &T) -> HttpResponse {
        Self::with_status(data, 201)
    }

    /// Return a 204 No Content response.
    pub fn no_content() -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

    /// Return a 400 Bad Request JSON response.
    pub fn bad_request(message: &str) -> HttpResponse {
        HttpResponse::BadRequest()
            .content_type("application/json")
            .body(serde_json::json!({"error": message}).to_string())
    }

    /// Return a 404 Not Found JSON response.
    pub fn not_found(message: &str) -> HttpResponse {
        HttpResponse::NotFound()
            .content_type("application/json")
            .body(serde_json::json!({"error": message}).to_string())
    }

    /// Return a 500 Internal Server Error JSON response.
    pub fn error(message: &str) -> HttpResponse {
        HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(serde_json::json!({"error": message}).to_string())
    }
}
