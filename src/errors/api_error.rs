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
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Configuration(e) => ApiError {
                code: ErrorCode::Configuration,
                message: format!("Database configuration error: {e}"),
            },
            sqlx::Error::Database(e) => ApiError {
                code: ErrorCode::Database,
                message: format!("Database error: {e}"),
            },
            sqlx::Error::Io(e) => ApiError {
                code: ErrorCode::Io,
                message: format!("I/O error: {e}"),
            },
            sqlx::Error::Tls(e) => ApiError {
                code: ErrorCode::Tls,
                message: format!("TLS error: {e}"),
            },
            sqlx::Error::Protocol(e) => ApiError {
                code: ErrorCode::Protocol,
                message: format!("Protocol error: {e}"),
            },
            sqlx::Error::RowNotFound => ApiError {
                code: ErrorCode::RowNotFound,
                message: "The requested row was not found".to_string(),
            },
            sqlx::Error::TypeNotFound { type_name } => ApiError {
                code: ErrorCode::TypeNotFound,
                message: format!("Type not found: {type_name}"),
            },
            sqlx::Error::ColumnIndexOutOfBounds { index, len } => ApiError {
                code: ErrorCode::ColumnIndexOutOfBounds,
                message: format!("Column index {index} is out of bounds (columns: {len})"),
            },
            sqlx::Error::ColumnNotFound(column) => ApiError {
                code: ErrorCode::ColumnNotFound,
                message: format!("Column not found: {column}"),
            },
            sqlx::Error::ColumnDecode { index, source } => ApiError {
                code: ErrorCode::ColumnDecode,
                message: format!("Failed to decode column {index}: {source}"),
            },
            sqlx::Error::Decode(e) => ApiError {
                code: ErrorCode::Decode,
                message: format!("Decode error: {e}"),
            },
            sqlx::Error::PoolTimedOut => ApiError {
                code: ErrorCode::PoolTimedOut,
                message: "Connection pool timed out".to_string(),
            },
            sqlx::Error::PoolClosed => ApiError {
                code: ErrorCode::PoolClosed,
                message: "Connection pool is closed".to_string(),
            },
            sqlx::Error::WorkerCrashed => ApiError {
                code: ErrorCode::WorkerCrashed,
                message: "Database worker thread crashed".to_string(),
            },
            sqlx::Error::Migrate(e) => ApiError {
                code: ErrorCode::Migrate,
                message: format!("Migration error: {e}"),
            },
            sqlx::Error::InvalidArgument(e) => ApiError {
                code: ErrorCode::InvalidArgument,
                message: format!("Invalid argument: {e}"),
            },
            sqlx::Error::Encode(e) => ApiError {
                code: ErrorCode::Encode,
                message: format!("Encode error: {e}"),
            },
            sqlx::Error::AnyDriverError(e) => ApiError {
                code: ErrorCode::AnyDriverError,
                message: format!("Driver error: {e}"),
            },
            sqlx::Error::InvalidSavePointStatement => ApiError {
                code: ErrorCode::InvalidSavePointStatement,
                message: "Invalid save point statement".to_string(),
            },
            sqlx::Error::BeginFailed => ApiError {
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
