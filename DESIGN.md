# floz — Design Document

> A batteries-included MVC web framework for Rust.
> Built on ntex + Floz ORM — convention over configuration, like Django/Rails for Rust.

---

## 1. Philosophy

**"Batteries included, no black-box magic."**

The Rust web ecosystem is fragmented: you assemble a web server, ORM, auth layer, background
workers, and logging from scratch every time. `floz` takes the Django/Rails approach —
a opinionated, cohesive framework where everything works together out of the box.

Unlike Loco.rs (which wraps Axum + SeaORM), `floz` is built on **ntex** (a high-performance
async web framework) and **Floz ORM** (a custom proc-macro-based ORM with dirty tracking). This
gives you tight integration from the database to the HTTP layer without stitching third-party
crates together.

### Core Design Principles

1. **Convention over Configuration** — sensible defaults, override only what you need
2. **Feature-Gated Modularity** — opt-in to auth, workers, OpenAPI via Cargo features
3. **ORM-First** — Floz ORM is a first-class citizen, not an afterthought
4. **Ergonomic Macros** — `res!()`, `pp!()`, `echo!()`, `xquery!()` reduce boilerplate
5. **Environment-Driven Config** — `.env` + `Config::from_env()`, no YAML/TOML files

### Non-Goals

- Being framework-agnostic (floz is opinionated about ntex)
- Supporting every database (PostgreSQL only, via SQLx)
- Replacing raw SQL for analytics queries (use `xquery!()` or `execute_query()`)

---

## 2. Quick Start

```rust
use floz::prelude::*;

#[route(
    get: "/users",
    tag: "Users",
    desc: "List all users",
    resps: [(200, "User list")],
)]
async fn list_users(ctx: web::types::State<AppContext>) -> HttpResponse {
    let db = floz::Db::from_pool((*ctx.db_pool).clone());
    let users = User::all(&db).await.unwrap();
    res!(pp!(&users).unwrap_or_default())
}

#[route(
    get: "/users/:id",
    tag: "Users",
    desc: "Get user by ID",
    resps: [
        (200, "User found"),
        (404, "User not found"),
    ],
)]
async fn get_user(
    path: web::types::Path<i32>,
    ctx: web::types::State<AppContext>,
) -> HttpResponse {
    let db = floz::Db::from_pool((*ctx.db_pool).clone());
    match User::get(path.into_inner(), &db).await {
        Ok(user) => res!(pp!(&user).unwrap_or_default()),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    App::new().run().await   // auto-discovers all #[route] handlers
}
```

### Environment (`.env`)

```
DATABASE_URL=postgres://user:pass@localhost:5432/mydb
HOST=127.0.0.1
PORT=3030
SERVER_ENV=DEV
```

---

## 3. Architecture Overview

### 3.1 Crate Layout

```
floz/src/
├── lib.rs                # Module tree + re-exports
├── prelude.rs            # use floz::prelude::*;
│
├── app/                  # Application bootstrap
│   ├── boot.rs           # App builder (config, hooks, auto-discovery)
│   └── context.rs        # AppContext (shared state)
│
├── router.rs             # #[route] auto-discovery, OpenAPI gen, Swagger UI
├── config.rs             # Environment-driven configuration
├── server.rs             # ServerConfig (bind address)
├── macros.rs             # echo!, res!, pp!, xquery!, to_json!
│
├── db/                   # Database layer
│   ├── pool.rs           # SQLx connection pool
│   └── query.rs          # Dynamic SQL executor
│
├── errors/               # Error types
│   └── api_error.rs      # ApiError + ErrorCode
│
├── controller/           # Controller utilities
│   ├── format.rs         # JsonResponse helper
│   └── pagination.rs     # PaginationParams
│
├── middleware/            # HTTP middleware
│   └── cors.rs           # CORS middleware
│
├── auth/                 # 🔒 Feature: auth
│   ├── jwt.rs            # JWT create/verify
│   └── api_key.rs        # API key generation
│
├── logger/               # 🔒 Feature: logger
│   ├── tracing_init.rs   # Daily-rotating tracing setup
│   └── http_logger.rs    # Request/response logging
│
├── openapi/              # 🔒 Feature: openapi
│   └── swagger.rs        # utoipa + Swagger UI
│
└── worker/               # 🔒 Feature: worker
    └── (Phase 3)         # Background tasks, queues, PG LISTEN
```

### 3.2 Feature Flags

```toml
[features]
default = ["full"]
full    = ["auth", "worker", "logger", "openapi", "mailer", "storage"]
auth    = ["dep:jsonwebtoken", "dep:bcrypt", "dep:sha2", "dep:base64"]
worker  = ["dep:redis"]
logger  = ["dep:tracing-appender"]
openapi = ["dep:utoipa"]
mailer  = []
storage = []
```

| Feature   | What It Adds                                      | Extra Dependencies               |
|-----------|---------------------------------------------------|----------------------------------|
| `auth`    | JWT, API keys, bcrypt hashing                     | jsonwebtoken, bcrypt, sha2       |
| `worker`  | Background task manager, Redis queues             | redis                            |
| `logger`  | Daily-rotating file logs via tracing              | tracing-appender                 |
| `openapi` | utoipa + Swagger UI auto-generation               | utoipa                           |
| `mailer`  | Email transport abstraction (placeholder)         | —                                |
| `storage` | File storage abstraction (placeholder)            | —                                |

### 3.3 Dependency Tree

```
floz
├── floz (ORM — workspace member)
├── floz-macros (#[route] proc macro — workspace member)
├── inventory (auto-discovery at runtime)
├── ntex (HTTP server)
├── sqlx (database driver)
├── tokio (async runtime)
├── serde + serde_json (serialization)
├── tracing + tracing-subscriber (logging)
├── chrono, uuid (common types)
├── thiserror, anyhow (error handling)
├── validator (input validation)
├── dashmap (concurrent maps)
└── [feature-gated]
    ├── jsonwebtoken + bcrypt + sha2 (auth)
    ├── redis (worker)
    ├── tracing-appender (logger)
    └── utoipa (openapi)
```

---

## 4. App Module — Application Bootstrap

### 4.1 App Builder

The `App` struct follows the builder pattern to configure and launch your application.
All handlers annotated with `#[route(...)]` are auto-discovered — no manual route registration needed.

```rust
// Minimal — everything auto-configured
App::new().run().await

// With options
App::new()
    .config(Config::from_env())           // optional — auto-loads .env
    .server(ServerConfig::new()           // optional — defaults to HOST:PORT
        .with_default_port(8080))
    .on_boot(|ctx| {                      // optional — runs after DB init
        info!("Booted with {} connections", ctx.db_pool.size());
    })
    .run()                                // starts the HTTP server
    .await
```

**Internal Flow:**
```
App::run()
  → Load Config (from_env or explicit)
  → Initialize tracing (if logger feature)
  → Create AppContext (DB pool + config)
  → Auto-discover all #[route(...)] handlers via inventory
  → Print route table (in dev mode or FLOZ_PRINT_ROUTES=1)
  → Fire on_boot hook
  → Generate OpenAPI spec from route metadata
  → Start ntex HttpServer
    → Mount /docs (Swagger UI) and /api-docs/openapi.json
    → Auto-register all discovered handlers
    → Bind to socket address
    → Serve requests
```

### 4.2 AppContext

Shared application state passed to every handler via ntex's state system:

```rust
pub struct AppContext {
    pub db_pool: DbPool,    // Arc<Pool<Postgres>>
    pub config: Config,      // Environment config
}
```

Access in handlers:

```rust
#[route(get: "/users", tag: "Users", desc: "List all users")]
async fn list_users(ctx: web::types::State<AppContext>) -> HttpResponse {
    let db = floz::Db::from_pool((*ctx.db_pool).clone());
    // ...
}
```

---

## 5. Configuration

### 5.1 Config Struct

All configuration is loaded from environment variables:

```rust
pub struct Config {
    // Required
    pub database_url: String,      // DATABASE_URL
    pub host: String,              // HOST (default: 127.0.0.1)
    pub port: String,              // PORT (default: 3030)
    pub server_env: String,        // SERVER_ENV (default: DEV)

    // Optional
    pub redis_url: Option<String>,       // REDIS_URL
    pub jwt_secret: Option<String>,      // JWT_TOKEN
    pub jwt_audience: Option<String>,    // JWT_AUDIENCE
    pub jwt_issuer: Option<String>,      // JWT_ISSUER
    pub echo: bool,                      // ECHO (debug flag)
}
```

### 5.2 Usage Patterns

```rust
// Automatic — loads .env on first access
let config = Config::from_env();

// Global singleton
let config = Config::global();

// Environment helpers
if config.is_dev() { /* dev-only logic */ }
if config.is_prod() { /* prod-only logic */ }

// Custom env vars
let api_key = Config::require("MY_API_KEY");  // panics if missing
let optional = Config::get("MY_OPTIONAL");     // returns Option<String>
```

### 5.3 ServerConfig

Configurable bind address with sensible defaults:

```rust
// Default: reads HOST and PORT env vars
let addr = ServerConfig::default().get_socket_addr();

// Custom: for background services on different ports
let addr = ServerConfig::new()
    .with_port_key("BACKGROUND_PORT")
    .with_default_port(3031)
    .get_socket_addr();
```

---

## 6. Database Layer

### 6.1 Connection Pool

Automatic pool sizing based on CPU cores:

```rust
// Auto-sized pool (uses available_parallelism)
let pool: DbPool = pool(0).await;

// Explicit worker count
let pool: DbPool = pool(4).await;

// Full control
let pool: DbPool = pool_with_options(&PoolOptions {
    min_connections: 4,
    max_connections: 20,
    acquire_timeout_secs: 30,
    idle_timeout_secs: 600,
    max_lifetime_secs: 1800,
}).await;
```

**Pool Defaults:**

| Setting          | Default | Notes                       |
|------------------|---------|-----------------------------|
| min_connections  | 4       | Kept alive at all times     |
| max_connections  | 10      | Or CPU count, whichever is higher |
| acquire_timeout  | 20s     | Before PoolTimedOut error   |
| idle_timeout     | 300s    | Idle connections are reaped |
| max_lifetime     | 900s    | Connections are recycled    |

### 6.2 Dynamic Query Execution

For queries that can't use Floz's typesafe `schema!` macro:

```rust
// Deserialize into a typed struct
let users: Vec<User> = execute_query(
    "SELECT * FROM users WHERE age > 25".into(),
    &ctx.db_pool,
).await?;

// Get raw JSON string
let json: String = execute_query_json(
    "SELECT * FROM analytics".into(),
    &ctx.db_pool,
).await?;

// Single row
let user: User = execute_one_query(
    format!("SELECT * FROM users WHERE id = {}", user_id),
    &ctx.db_pool,
).await?;
```

**How It Works:** Fetches `sqlx::Row`s → converts each column to `serde_json::Value` →
serializes to JSON string → deserializes into the target type `T`. This round-trip approach
handles arbitrary SQL without compile-time schema knowledge.

### 6.3 ORM Integration

Floz ORM is re-exported as a first-class dependency:

```rust
// In your models
use floz::prelude::*;

floz::schema! {
    model User("users") {
        id:    integer("id").auto_increment().primary(),
        name:  varchar("name", 100),
        email: varchar("email", 255).nullable(),
    }
}

// In your handlers — seamless integration
let db = floz::Db::from_pool((*ctx.db_pool).clone());
let users = User::all(&db).await?;
let user = User::get(1, &db).await?;
```

---

## 7. Error Handling

### 7.1 ApiError

A structured error type with automatic conversions from common error sources:

```rust
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}
```

### 7.2 ErrorCode Enum

```rust
pub enum ErrorCode {
    // General
    GenericError, BadRequest, NotFound, Forbidden,
    InternalServerError, TooManyRequests,

    // Database (mapped from sqlx::Error variants)
    DatabaseError, Configuration, Database, Io, Tls,
    Protocol, RowNotFound, TypeNotFound,
    ColumnIndexOutOfBounds, ColumnNotFound, ColumnDecode,
    Encode, Decode, PoolTimedOut, PoolClosed,
    WorkerCrashed, Migrate, BeginFailed,

    // Auth
    JwtError, InvalidUUID,

    // Processing
    ProcessingError,
}
```

### 7.3 Automatic Conversions

```rust
// All of these convert automatically via From<T>:
let err: ApiError = sqlx_error.into();           // Every sqlx::Error variant is mapped
let err: ApiError = serde_json_error.into();     // → BadRequest
let err: ApiError = uuid_error.into();           // → InvalidUUID
let err: ApiError = anyhow_error.into();         // → GenericError
let err: ApiError = jwt_error.into();            // → specific JWT error (auth feature)
let err: ApiError = redis_error.into();          // → DatabaseError (worker feature)

// Convenience constructors
let err = ApiError::bad_request("Invalid email format");
let err = ApiError::not_found("User not found");
let err = ApiError::forbidden("Insufficient permissions");
let err = ApiError::internal("Something went wrong");
```

---

## 8. Controller Utilities

### 8.1 JsonResponse

Structured response helpers with environment-aware formatting:

```rust
// Pretty-prints in DEV, compact in PROD
JsonResponse::ok(&users)              // 200 OK
JsonResponse::created(&user)          // 201 Created
JsonResponse::no_content()            // 204 No Content
JsonResponse::with_status(&data, 202) // Custom status

// Error responses
JsonResponse::bad_request("Invalid input")
JsonResponse::not_found("User not found")
JsonResponse::error("Internal server error")
```

### 8.2 PaginationParams

Standardized pagination extracted from query strings:

```rust
#[route(get: "/users", tag: "Users", desc: "List users with pagination")]
async fn list(params: web::types::Query<PaginationParams>) -> HttpResponse {
    let page = params.into_inner();
    // page.limit     → 10 (default)
    // page.offset    → 0 (default)
    // page.order_by  → "created" (default)
    // page.filter    → "" (e.g. "status:active,role:admin")
    // page.search    → "" (e.g. "alice")
}
```

**Supported URL Patterns:**
```
/users?limit=20&offset=0&order_by=created
/users?limit=10&offset=40&order_by=name&search=alice
/users?limit=10&offset=0&filter=status:active,role:admin
```

---

## 9. Middleware

### 9.1 CORS

Full CORS middleware with builder pattern:

```rust
use floz::middleware::cors::Cors;

// Permissive — allows all origins (development)
App::new()
    .wrap(Cors::permissive())

// Restrictive — production configuration
App::new()
    .wrap(Cors::new()
        .allow_origin("https://example.com")
        .allow_origin("https://api.example.com")
        .allow_methods(["GET", "POST", "PUT", "DELETE"])
        .allow_headers(vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
        ])
        .expose_headers(vec![header::CONTENT_DISPOSITION])
        .supports_credentials()
        .max_age(86400)
    )
```

**Permissive defaults:**
- Methods: GET, POST, PUT, DELETE, PATCH, OPTIONS
- Headers: Authorization, Accept, Content-Type
- Credentials: enabled
- Max age: 3600s

**Behavior:**
- Preflight `OPTIONS` requests are intercepted and responded to immediately
- Actual requests get CORS headers applied after the inner service responds
- Unknown origins return `403 Forbidden` when origins are explicitly restricted
- Empty origin set = allow all (wildcard mode)

---

## 10. Auth Module (`auth` feature)

### 10.1 JWT

Token creation and verification with HS256:

```rust
use floz::auth::jwt;

// Create
let (token, expiry_secs) = jwt::create_token(
    "user-123",           // subject (user ID)
    "admin",              // role
    b"my-secret",         // signing key
    "my-app",             // audience
    "floz",              // issuer
    24,                   // expiry in hours
)?;

// Verify
let claims = jwt::verify_token(
    &token,
    b"my-secret",
    "my-app",             // expected audience
    "floz",              // expected issuer
)?;
// claims.sub → "user-123"
// claims.role → "admin"
// claims.exp → Unix timestamp
```

### 10.2 API Keys

Secure key generation and bcrypt-based verification:

```rust
use floz::auth::api_key;

// Generate with custom prefix
let key = api_key::generate_api_key("sk");
// → "sk_V1StGXR8_Z5jdHi6B-myT" (21-char nanoid)

// Hash for storage (bcrypt)
let hash = api_key::hash_api_key(&key)?;

// Verify
let valid = api_key::verify_api_key_hash(&key, &hash)?;
```

---

## 11. Macros

### 11.1 `echo!()` — Debug Logging

Only prints when the `ECHO` environment variable is set:

```rust
echo!("Processing user {}", user.id);
echo!(data = ?request_body, "Incoming request");
// Silent in production (ECHO not set)
// Prints via tracing::info!() when ECHO=1
```

### 11.2 `res!()` — Quick JSON Response

```rust
// 200 OK
res!(body)

// Custom status
res!(body, 201)

// Typical usage
res!(pp!(&users).unwrap_or_default())
res!(serde_json::to_string(&data)?, 201)
```

### 11.3 `pp!()` — Pretty Print

Environment-aware JSON serialization:

```rust
let json = pp!(&data).unwrap_or_default();
// DEV  → to_string_pretty (indented)
// PROD → to_string (compact)
```

### 11.4 `xquery!()` — SQLx Query Shorthand

```rust
// Simple
xquery!("SELECT * FROM users");

// With parameters (auto-binds)
xquery!("SELECT * FROM users WHERE id = $1 AND name = $2", user_id, name);
// Expands to: sqlx::query(...).bind(user_id).bind(name)
```

### 11.5 `to_json!()` — Row to JSON Map

Converts a `sqlx::Row` to a `serde_json::Map`, handling common PostgreSQL types:

```rust
let row = xquery!("SELECT * FROM users WHERE id = 1")
    .fetch_one(pool)
    .await?;
let map = to_json!(row);
// Handles: UUID, TEXT, VARCHAR, INT4, INT8, FLOAT4, FLOAT8,
//          BOOL, TIMESTAMPTZ, TIMESTAMP
```

---

## 12. Logging (`logger` feature)

### 12.1 Tracing Initialization

Daily-rotating log files with stdout mirroring:

```rust
// Called automatically by App::run() when logger feature is enabled
// Or call manually:
floz::logger::init_tracing();
```

**Behavior:**
- Logs to `stdout` AND `./logs/<binary_name>.log`
- Daily rotation — new file each day
- Non-blocking writes (background thread)
- Default filter: `debug` (override via `RUST_LOG` env var)
- Safe to call multiple times (uses `Once` guard)

**Output:**
```
logs/
├── my-app.log.2026-04-01
├── my-app.log.2026-04-02
└── my-app.log.2026-04-03  ← current
```

---

## 13. Route System — `#[route(...)]`

The route system is the heart of floz. A single `#[route(...)]` attribute on every handler defines **everything** — HTTP method, URL, OpenAPI metadata, auth, rate limits, middleware. Handlers auto-register themselves via `inventory` — no manual route wiring.

### 13.1 Basic Usage

```rust
use floz::prelude::*;

#[route(
    get: "/users/:id",
    tag: "Users",
    desc: "Get a user by ID",
    resps: [
        (200, "User found", Json<User>),
        (404, "User not found"),
    ],
)]
async fn get_user(path: web::types::Path<i32>) -> HttpResponse {
    let id = path.into_inner();
    HttpResponse::Ok().json(&json!({ "id": id, "name": "Alice" }))
}
```

### 13.2 Attribute Fields

| Field | Required | Example | Purpose |
|-------|----------|---------|---------|
| `get:` / `post:` / `put:` / `patch:` / `delete:` | ✓ | `get: "/users"` | HTTP method + path |
| `tag:` | | `tag: "Users"` | OpenAPI grouping + route table |
| `desc:` | | `desc: "List all users"` | OpenAPI description |
| `resps:` | | `[(200, "OK", Json<User>)]` | Response status + desc + optional schema |
| `auth:` | | `auth: jwt` | Per-route auth requirement |
| `rate:` | | `rate: "100/min"` | Per-route rate limit |
| `wrap:` | | `wrap: [Logger::default()]` | Per-route middleware |

### 13.3 Path Parameters

Use Express-style `:param` syntax — the macro auto-translates to ntex's `{param}` internally and OpenAPI's `{param}` in the spec:

```rust
#[route(get: "/posts/:post_id/comments/:comment_id")]
async fn get_comment(
    path: web::types::Path<(i32, i32)>,
) -> HttpResponse {
    let (post_id, comment_id) = path.into_inner();
    // ...
}
```

### 13.4 Response Schemas (OpenAPI)

Response specs can include typed schemas for automatic OpenAPI documentation:

```rust
#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[route(
    get: "/users/:id",
    resps: [
        (200, "User found", Json<User>),     // generates schema in OpenAPI
        (404, "Not found"),                   // no schema
    ],
)]
```

### 13.5 Per-Route Middleware

Apply ntex middleware to individual routes using `wrap:`:

```rust
#[route(
    get: "/admin/stats",
    tag: "Admin",
    wrap: [ntex::web::middleware::Logger::default()],
)]
async fn admin_stats() -> HttpResponse {
    HttpResponse::Ok().body("stats")
}
```

### 13.6 How Auto-Discovery Works

```
                    compile time                     runtime
                   ┌────────────┐               ┌──────────────┐
#[route(..)]  ──►  proc macro  ──►  inventory::submit!  ──► App::run() iterates
  on fn login()   │  generates:  │    │ RouteEntry {       │ inventory::iter::<RouteEntry>
                   │  • handler   │    │   method: "get",   │ and registers each with
                   │  • submit!   │    │   path: "/login",  │ ntex's ServiceConfig
                   └────────────┘    │   handler: fn,     │
                                      │   tag: "Auth",     │
                                      │   ... }            │
                                      └────────────────────┘
```

1. `#[route]` proc macro parses the attribute fields
2. Generates the bare handler function + a registration function
3. Generates an `inventory::submit!` call that registers a `RouteEntry` metadata struct
4. `App::run()` iterates `inventory::iter::<RouteEntry>` and calls `cfg.service()` for each
5. Also generates the OpenAPI spec from the collected route metadata

### 13.7 Auto-Generated Endpoints

`App::run()` automatically mounts:

| Endpoint | Purpose |
|----------|---------|
| `/docs` | Swagger UI (interactive API docs) |
| `/api-docs/openapi.json` | Raw OpenAPI 3.0 JSON spec |

### 13.8 Route Table

In dev mode (or when `FLOZ_PRINT_ROUTES=1` is set), `App::run()` prints all discovered routes:

```
  METHOD  PATH                         TAG                AUTH   RATE       DESCRIPTION
  ──────  ────────────────────────────  ──────────────────  ──────  ──────────  ───────────────────────────────
  GET     /auth/login                  Auth: Session      —      —          Serve the login page
  POST    /users                       Users              jwt    —          Create a new user
  GET     /users/:id                   Users              —      —          Get user by ID
  GET     /health                      System             —      —          Health check
```

---

## 14. Prelude

The prelude re-exports everything you need for common usage:

```rust
use floz::prelude::*;

// This gives you:
// App, AppContext, Config, ServerConfig
// ApiError, ErrorCode
// DbPool, pool
// PaginationParams, JsonResponse
// Cors, HttpLogger (if logger feature)
// floz::prelude::* (full ORM)
// floz_macros::route              ← #[route(...)] attribute macro
// echo!, res!, pp!, xquery!
// ntex::web::{self, HttpResponse, HttpRequest}
// serde::{Deserialize, Serialize}
// serde_json::{json, Value}
// tracing::{info, warn, error, debug, trace}
```

---

## 15. Project Starters

### 15.1 Minimal

Single-file server with health endpoint:

```
my-app/
├── Cargo.toml
├── .env
└── src/
    └── main.rs         # App::new().run().await + #[route] handlers
```

```rust
// src/main.rs
use floz::prelude::*;

#[route(get: "/health", tag: "System", desc: "Health check")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    App::new().run().await
}
```

### 15.2 API (recommended)

Modular domain structure:

```
my-app/
├── Cargo.toml
├── .env
└── src/
    ├── main.rs         # App::new().run().await (no route registration)
    └── app/
        ├── mod.rs      # Module registry (just `pub mod user; pub mod post;`)
        ├── user/       # Domain module (floz generate scaffold User)
        │   ├── mod.rs
        │   ├── model.rs
        │   └── route.rs    # #[route] handlers — auto-discovered
        └── post/
            ├── mod.rs
            ├── model.rs
            └── route.rs    # #[route] handlers — auto-discovered
```

### 15.3 SaaS

Full features with auth + background workers:

```
my-app/
├── Cargo.toml          # floz with features = ["full"]
├── .env                # Includes REDIS_URL, JWT_TOKEN, etc.
└── src/
    ├── main.rs         # App::new().on_boot(...).run().await
    └── app/
        └── ...
```

---

## 16. CLI Tool

The `floz` CLI scaffolds projects and generates code:

```bash
# Create a new project
floz new my-app --template api

# Generate domain modules
floz generate model Post title:string body:text author_id:integer
floz generate controller posts
floz generate scaffold Post title:string body:text

# Generated output:
# src/app/post/
#   ├── mod.rs     → pub mod model; pub mod route;
#   ├── model.rs   → floz::schema! { model Post("posts") { ... } }
#   └── route.rs   → #[route(get: "/posts")] + #[route(get: "/posts/:id")]

# Database (coming soon)
floz db migrate
floz db rollback
floz db seed
```

### Supported Field Types

| CLI Type   | Floz Macro            | Rust Type           | PostgreSQL          |
|------------|------------------------|---------------------|---------------------|
| `string`   | `varchar("col", 255)`  | `String`            | `VARCHAR(255)`      |
| `text`     | `text("col")`          | `String`            | `TEXT`              |
| `integer`  | `integer("col")`       | `i32`               | `INTEGER`           |
| `bigint`   | `bigint("col")`        | `i64`               | `BIGINT`            |
| `short`    | `short("col")`         | `i16`               | `SMALLINT`          |
| `bool`     | `bool("col")`          | `bool`              | `BOOLEAN`           |
| `uuid`     | `uuid("col")`          | `Uuid`              | `UUID`              |
| `datetime` | `datetime("col").tz()` | `DateTime<Utc>`     | `TIMESTAMPTZ`       |
| `date`     | `date("col")`          | `NaiveDate`         | `DATE`              |
| `float`    | `real("col")`          | `f32`               | `REAL`              |
| `double`   | `double("col")`        | `f64`               | `DOUBLE PRECISION`  |
| `json`     | `json("col")`          | `serde_json::Value` | `JSON`              |
| `jsonb`    | `jsonb("col")`         | `serde_json::Value` | `JSONB`             |

---

## 17. How It Compares

| Feature             | Loco.rs              | floz                                  |
|---------------------|----------------------|----------------------------------------------|
| HTTP Framework      | Axum                 | ntex                                         |
| ORM                 | SeaORM               | **Floz** (custom proc-macro, dirty tracking)|
| Route Definition    | manual `.route()`    | **`#[route(...)]` auto-discovery**           |
| Auth                | Sessions + JWT       | API key + JWT + Cookie + bcrypt              |
| Background Jobs     | SidekiqRS / Tokio    | BackgroundTaskManager + Redis queue          |
| PG Events           | ❌                    | PG LISTEN/NOTIFY bridge                      |
| CLI                 | `cargo loco`         | `floz` CLI                                  |
| Event Queue         | ❌                    | EventService + EventProcessor                |
| OpenAPI             | ❌ built-in           | **auto-generated from `#[route]` metadata**  |
| Task Scheduler      | Background workers   | Full TaskQueue + Worker + Client             |
| SQL Query Parser    | ❌                    | Dynamic query executor                       |
| Error Handling      | Basic                | 30+ error codes with auto-conversion         |

---

## 18. Roadmap

### Implemented ✅

- [x] App builder with config + boot hooks
- [x] `#[route(...)]` proc macro — single-annotation route + OpenAPI + auth + rate limiting
- [x] Auto-discovery via `inventory` — no manual route registration
- [x] OpenAPI auto-generation from route metadata + Swagger UI at `/docs`
- [x] Route table printing (`FLOZ_PRINT_ROUTES=1` or dev mode)
- [x] Per-route middleware via `wrap: [...]`
- [x] AppContext with DB pool + config
- [x] Environment-driven configuration
- [x] SQLx connection pool with auto-sizing
- [x] Dynamic SQL query executor
- [x] Floz ORM integration (re-exported)
- [x] ApiError + ErrorCode with broad `From` conversions
- [x] CORS middleware (full RFC 6454 compliance)
- [x] JWT token creation/verification
- [x] API key generation + bcrypt hashing
- [x] Tracing with daily log rotation
- [x] JsonResponse helpers
- [x] PaginationParams
- [x] Utility macros (echo!, res!, pp!, xquery!, to_json!)
- [x] CLI scaffolding tool
- [x] Project starter templates (minimal, api, saas)

### In Progress 🔄

- [ ] Background worker extraction (Phase 3)
- [ ] Event queue + Redis cache
- [ ] PG LISTEN/NOTIFY bridge
- [ ] HTTP request/response logger middleware

### Planned 📋

- [ ] Migration runner (`floz db migrate`)
- [ ] Rate limiting middleware enforcement
- [ ] Auth middleware enforcement (`auth: jwt` / `auth: api_key`)
- [ ] Mailer transport abstraction
- [ ] File storage abstraction (OpenDAL)
- [ ] `floz routes` CLI command
- [ ] REPL console (`floz console`)
- [ ] Compile-time response type validation
