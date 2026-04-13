//! Test request and response types.
//!
//! Built on ntex's `ClientRequest` / `ClientResponse` — zero external deps.

use ntex::http::Method;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

/// A builder for test HTTP requests.
///
/// Created by `TestApp::get()`, `TestApp::post()`, etc.
///
/// # Example
///
/// ```ignore
/// let resp = app.post("/users")
///     .json(&json!({ "name": "Alice", "email": "alice@example.com" }))
///     .header("X-Custom", "value")
///     .send()
///     .await;
/// ```
pub struct TestRequest<'a> {
    app: &'a super::TestApp,
    method: &'a str,
    path: String,
    body: Option<serde_json::Value>,
    headers: HashMap<String, String>,
    query_string: Option<String>,
}

impl<'a> TestRequest<'a> {
    pub(crate) fn new(app: &'a super::TestApp, method: &'a str, path: &str) -> Self {
        Self {
            app,
            method,
            path: path.to_string(),
            body: None,
            headers: HashMap::new(),
            query_string: None,
        }
    }

    /// Set a JSON request body.
    pub fn json(mut self, body: &serde_json::Value) -> Self {
        self.body = Some(body.clone());
        self
    }

    /// Add a request header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the Authorization Bearer token.
    pub fn bearer(self, token: &str) -> Self {
        self.header("Authorization", format!("Bearer {}", token))
    }

    /// Add query parameters from key-value pairs.
    pub fn query(mut self, params: &[(&str, &str)]) -> Self {
        let qs = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        self.query_string = Some(qs);
        self
    }

    /// Send the request and return the response.
    pub async fn send(self) -> TestResponse {
        let method = match self.method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "PATCH" => Method::PATCH,
            "DELETE" => Method::DELETE,
            _ => Method::GET,
        };

        let path = if let Some(qs) = &self.query_string {
            format!("{}?{}", self.path, qs)
        } else {
            self.path.clone()
        };

        // Use ntex TestServer's request() to get a ClientRequest
        let mut req = self.app.server.request(method, self.app.server.url(&path));

        // Add default headers from TestApp
        for (k, v) in &self.app.default_headers {
            req = req.header(k.as_str(), v.as_str());
        }

        // Add request-specific headers
        for (k, v) in &self.headers {
            req = req.header(k.as_str(), v.as_str());
        }

        // Send with or without body
        let response = if let Some(body) = self.body {
            let body_bytes = serde_json::to_vec(&body).unwrap();
            req.header("content-type", "application/json")
                .send_body(body_bytes)
                .await
                .expect("Failed to send test request")
        } else {
            req.send().await.expect("Failed to send test request")
        };

        // Eagerly read all response data — no borrowing needed downstream
        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.to_string(), val.to_string())))
            .collect();
        let body_bytes = self
            .app
            .server
            .load_body(response)
            .await
            .expect("Failed to read response body");

        TestResponse {
            status,
            headers,
            body: body_bytes.to_vec(),
        }
    }
}

/// A wrapper around the HTTP response from a test request.
///
/// All data is eagerly loaded — no borrowing, no async needed to read.
///
/// # Example
///
/// ```ignore
/// let resp = app.get("/users").send().await;
/// assert_eq!(resp.status(), 200);
///
/// let body = resp.json_value();
/// assert!(body.is_array());
/// ```
pub struct TestResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl TestResponse {
    /// Get the HTTP status code as a u16.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Check if the response was successful (2xx).
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Get a response header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Read the response body as a deserialized JSON value.
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body).expect("Failed to deserialize response body as JSON")
    }

    /// Read the response body as raw text.
    pub fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("Response body is not valid UTF-8")
    }

    /// Read the response body as a `serde_json::Value`.
    pub fn json_value(&self) -> serde_json::Value {
        self.json::<serde_json::Value>()
    }

    /// Get the raw body bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}
