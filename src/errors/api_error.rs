use serde::Serialize;
use serde_json::Error as SerdeJsonError;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Error Codes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum ErrorCode {
    // General
    GenericError,
    BadRequest,
    NotFound,
    Forbidden,
    InternalServerError,
    TooManyRequests,

    // UUID
    InvalidUUID,

    // Database
    DatabaseError,
    Configuration,
    Database,
    Io,
    Tls,
    Protocol,
    RowNotFound,
    TypeNotFound,
    ColumnIndexOutOfBounds,
    ColumnNotFound,
    ColumnDecode,
    Encode,
    Decode,
    AnyDriverError,
    PoolTimedOut,
    PoolClosed,
    WorkerCrashed,
    Migrate,
    InvalidSavePointStatement,
    BeginFailed,
    InvalidArgument,

    // Auth
    JwtError,

    // Processing
    ProcessingError,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// ApiError
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        ApiError {
            code,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalServerError, message)
    }

    pub fn code(&self) -> &ErrorCode {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl ntex::web::WebResponseError for ApiError {
    fn error_response(&self, _: &ntex::web::HttpRequest) -> ntex::web::HttpResponse {
        use ntex::http::StatusCode;

        let status_code = match self.code {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::InvalidUUID => StatusCode::BAD_REQUEST,
            ErrorCode::JwtError => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // In production, redact internal error details from 5xx responses
        // to prevent information leakage (table names, SQL errors, etc.)
        let is_prod = std::env::var("SERVER_ENV")
            .map(|v| v.eq_ignore_ascii_case("PROD"))
            .unwrap_or(false);

        if is_prod && status_code.is_server_error() {
            tracing::error!(code = ?self.code, message = %self.message, "Internal server error (redacted from response)");
            ntex::web::HttpResponse::build(status_code).json(&ApiError {
                code: ErrorCode::InternalServerError,
                message: "An internal error occurred.".to_string(),
            })
        } else {
            ntex::web::HttpResponse::build(status_code).json(&self)
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// From conversions
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        ApiError {
            code: ErrorCode::GenericError,
            message,
        }
    }
}

impl From<&str> for ApiError {
    fn from(message: &str) -> Self {
        ApiError {
            code: ErrorCode::GenericError,
            message: message.to_string(),
        }
    }
}

impl From<uuid::Error> for ApiError {
    fn from(err: uuid::Error) -> Self {
        ApiError {
            code: ErrorCode::InvalidUUID,
            message: err.to_string(),
        }
    }
}

impl From<SerdeJsonError> for ApiError {
    fn from(error: SerdeJsonError) -> Self {
        ApiError {
            code: ErrorCode::BadRequest,
            message: format!("JSON parse error: {error}"),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError {
            message: err.to_string(),
            code: ErrorCode::GenericError,
        }
    }
}

#[cfg(any(feature = "postgres", feature = "sqlite"))]
impl From<floz_orm::sqlx::Error> for ApiError {
    fn from(err: floz_orm::sqlx::Error) -> Self {
        match err {
            floz_orm::sqlx::Error::Configuration(e) => ApiError {
                code: ErrorCode::Configuration,
                message: format!("Database configuration error: {e}"),
            },
            floz_orm::sqlx::Error::Database(e) => ApiError {
                code: ErrorCode::Database,
                message: format!("Database error: {e}"),
            },
            floz_orm::sqlx::Error::Io(e) => ApiError {
                code: ErrorCode::Io,
                message: format!("I/O error: {e}"),
            },
            floz_orm::sqlx::Error::Tls(e) => ApiError {
                code: ErrorCode::Tls,
                message: format!("TLS error: {e}"),
            },
            floz_orm::sqlx::Error::Protocol(e) => ApiError {
                code: ErrorCode::Protocol,
                message: format!("Protocol error: {e}"),
            },
            floz_orm::sqlx::Error::RowNotFound => ApiError {
                code: ErrorCode::RowNotFound,
                message: "The requested row was not found".to_string(),
            },
            floz_orm::sqlx::Error::TypeNotFound { type_name } => ApiError {
                code: ErrorCode::TypeNotFound,
                message: format!("Type not found: {type_name}"),
            },
            floz_orm::sqlx::Error::ColumnIndexOutOfBounds { index, len } => ApiError {
                code: ErrorCode::ColumnIndexOutOfBounds,
                message: format!("Column index {index} is out of bounds (columns: {len})"),
            },
            floz_orm::sqlx::Error::ColumnNotFound(column) => ApiError {
                code: ErrorCode::ColumnNotFound,
                message: format!("Column not found: {column}"),
            },
            floz_orm::sqlx::Error::ColumnDecode { index, source } => ApiError {
                code: ErrorCode::ColumnDecode,
                message: format!("Failed to decode column {index}: {source}"),
            },
            floz_orm::sqlx::Error::Decode(e) => ApiError {
                code: ErrorCode::Decode,
                message: format!("Decode error: {e}"),
            },
            floz_orm::sqlx::Error::PoolTimedOut => ApiError {
                code: ErrorCode::PoolTimedOut,
                message: "Connection pool timed out".to_string(),
            },
            floz_orm::sqlx::Error::PoolClosed => ApiError {
                code: ErrorCode::PoolClosed,
                message: "Connection pool is closed".to_string(),
            },
            floz_orm::sqlx::Error::WorkerCrashed => ApiError {
                code: ErrorCode::WorkerCrashed,
                message: "Database worker thread crashed".to_string(),
            },
            floz_orm::sqlx::Error::Migrate(e) => ApiError {
                code: ErrorCode::Migrate,
                message: format!("Migration error: {e}"),
            },
            floz_orm::sqlx::Error::InvalidArgument(e) => ApiError {
                code: ErrorCode::InvalidArgument,
                message: format!("Invalid argument: {e}"),
            },
            floz_orm::sqlx::Error::Encode(e) => ApiError {
                code: ErrorCode::Encode,
                message: format!("Encode error: {e}"),
            },
            floz_orm::sqlx::Error::AnyDriverError(e) => ApiError {
                code: ErrorCode::AnyDriverError,
                message: format!("Driver error: {e}"),
            },
            floz_orm::sqlx::Error::InvalidSavePointStatement => ApiError {
                code: ErrorCode::InvalidSavePointStatement,
                message: "Invalid save point statement".to_string(),
            },
            floz_orm::sqlx::Error::BeginFailed => ApiError {
                code: ErrorCode::BeginFailed,
                message: "Failed to begin transaction".to_string(),
            },
            _ => ApiError {
                code: ErrorCode::DatabaseError,
                message: format!("Database error: {err:?}"),
            },
        }
    }
}

#[cfg(feature = "auth")]
impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => ApiError::bad_request("Token has expired"),
            ErrorKind::InvalidToken => ApiError::bad_request("Invalid token"),
            ErrorKind::InvalidIssuer => ApiError::bad_request("Invalid token issuer"),
            ErrorKind::InvalidAudience => ApiError::bad_request("Invalid token audience"),
            ErrorKind::InvalidSubject => ApiError::bad_request("Invalid token subject"),
            ErrorKind::ImmatureSignature => ApiError::bad_request("Token not yet valid"),
            ErrorKind::MissingRequiredClaim(claim) => {
                ApiError::bad_request(format!("Missing required claim: {claim}"))
            }
            _ => ApiError::bad_request(format!("JWT error: {err}")),
        }
    }
}

#[cfg(feature = "worker")]
impl From<redis::RedisError> for ApiError {
    fn from(error: redis::RedisError) -> Self {
        ApiError {
            code: ErrorCode::DatabaseError,
            message: format!("Redis error: {}", error),
        }
    }
}
