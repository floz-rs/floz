# Floz

A batteries-included Rust framework for building web APIs and tools on PostgreSQL.
Welcome to the **Floz** full-stack Rust web framework workspace!

This repository contains the `floz` core web framework, the integrated `floz-orm`, macros, CLI scaffolding utilities, and the terminal TUI database editor.

## Workspace

| Crate | Description |
|-------|-------------|
| **floz** | MVC web framework — built on ntex + floz-orm, convention over configuration |
| **examples** | Example binaries demonstrating the framework |
| **floz-orm** | Lightweight, typesafe ORM — unifying DAO and DSL from a single `schema!` macro |
| **floz-macros** | Proc macro engine — `schema!` parser/codegen + `#[route(...)]` attribute macro |
| **floz-cli** | CLI scaffolding tool — `floz new`, `floz generate`, project templates |
| **floz-editor** | Terminal-based PostgreSQL table editor — ratatui TUI for browsing/editing tables |

## Quick Start

### Web Framework

```rust
use floz::prelude::*;

#[route(get: "/users", tag: "Users", desc: "List all users")]
async fn list_users(ctx: web::types::State<AppContext>) -> HttpResponse {
    let db = floz::Db::from_pool((*ctx.db_pool).clone());
    let users = User::all(&db).await.unwrap();
    res!(pp!(&users).unwrap_or_default())
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    App::new().run().await   // auto-discovers all #[route] handlers
}
```

### ORM

```rust
use floz::prelude::*;

floz::schema! {
    model User("users") {
        id:    integer("id").auto_increment().primary(),
        name:  varchar("name", 100),
        email: text("email").unique(),
        age:   short("age").nullable(),
        active: bool("active"),
    }
}
```

This generates:
- `pub struct User` — with `id: i32`, `name: String`, `email: String`, `age: Option<i16>`, `active: bool`
- `pub struct UserTable` — with typed `Column<T>` constants for DSL queries
- DAO methods: `create()`, `get()`, `find()`, `save()`, `delete()`, `all()`, `filter()`
- Dirty-tracking setters: `set_name()`, `set_email()`, etc.

### CRUD Operations

```rust
// Connect
let db = Db::connect("postgres://user:pass@localhost/mydb").await?;

// Create
let user = User {
    name: "Alice".into(),
    email: "alice@example.com".into(),
    age: Some(30),
    active: true,
    ..User::default()
};
let alice = user.create(&db).await?; // INSERT ... RETURNING *

// Get by primary key
let user = User::get(alice.id, &db).await?;

// Find (returns Option)
let maybe_user = User::find(999, &db).await?; // Ok(None)

// Update (only dirty fields)
let mut user = User::get(1, &db).await?;
user.set_name("Alice Updated".into()); // marks name as dirty
user.set_age(Some(31));                // marks age as dirty
user.save(&db).await?;
// Generates: UPDATE users SET name = $1, age = $2 WHERE id = $3
// (email and active are NOT in the UPDATE — they weren't touched)

// Delete
user.delete(&db).await?;
```

### DSL Queries

```rust
// Type-safe column operators
let active_adults = User::filter(
    UserTable::active.eq(true)
        .and(UserTable::age.gte(18i16)),
    &db,
).await?;

// String operators
let search = User::filter(
    UserTable::name.contains("ali")
        .or(UserTable::email.ends_with("@company.com")),
    &db,
).await?;

// NULL checks
let no_age = User::filter(UserTable::age.is_null(), &db).await?;
```

### Transactions

```rust
let tx = db.begin().await?;

let user = User { name: "Bob".into(), ..User::default() };
let bob = user.create(&tx).await?;

let mut bob = User::get(bob.id, &tx).await?;
bob.set_active(false);
bob.save(&tx).await?;

tx.commit().await?;
// Or: tx.rollback().await? — also auto-rollbacks on drop
```

## Running the CLI

The `floz-cli` tool provides a convenient way to scaffold new projects and generate application components. You can execute it via cargo:

```bash
# General help
cargo run -p floz-cli -- --help

# Specific command help
cargo run -p floz-cli -- new --help
```

### Common CLI Tasks

- **Create a new project (default API template):**
  ```bash
  cargo run -p floz-cli -- new my_app
  ```

- **Create a new minimal project:**
  ```bash
  cargo run -p floz-cli -- new my_app --template minimal
  ```

- **Generate a new model inside an existing project:**
  ```bash
  cargo run -p floz-cli -- generate model Post title:string content:text user_id:integer
  ```

- **Generate a full scaffold (Model + Controller + Routes):**
  ```bash
  cargo run -p floz-cli -- generate scaffold Post title:string content:text
  ```

## Recommended Directory Structure

One of the super-powers of Floz's macro-driven system is that **it does not care what folder structure you use**. Whether you prefer classic MVC (`models/`, `controllers/`, `routes/`) or flat files, it will magically auto-discover your schema and routes anyway! 

However, to prevent you from getting stuck as your application scales, we strongly recommend a **Django-inspired "Apps" (Domain-Driven) layout**. When you scaffold a project using `floz new`, your application follows this standardized layout:

```text
my_app/
├── Cargo.toml            # Project configuration and dependencies
├── .env                  # Environment variables (DB URL, JWT secrets)
└── src/
    ├── main.rs           # Bootstraps the floz::App and runs `#[floz::main]`
    ├── middleware/       # Shared custom application middleware
    │   ├── mod.rs        
    │   ├── auth.rs       
    │   └── tenant.rs     
    └── app/              # Your Django-style modular "Apps"
        ├── mod.rs        # Auto-discovers all app modules automatically
        │
        ├── user/         # The "User" app module
        │   ├── mod.rs    
        │   ├── model.rs  # Schema definitions (Django's models.py)
        │   ├── routes.rs # HTTP HTTP Handlers (Django's views.py / action logic)
        │   ├── config.rs # (Optional) App-specific config
        │   └── helper.rs # (Optional) App-specific utilities
        │
        └── org/          # The "Organization" / multi-tenant app module
            ├── mod.rs
            ├── model.rs
            └── routes.rs
```

### Why this structure?
- **Domain-Driven Design (Django Apps)**: Grouping files by entity (`user/`, `org/`) rather than technical type (`controllers/`, `models/`) makes large enterprise codebases significantly easier to navigate and maintain. When you delete the `user` feature, you just delete one folder!
- **Zero-Config Discovery**: By exposing your apps inside `src/app/mod.rs`, Floz's `#[route]` macro automatically scans everything traversing downwards at compile-time. You never have to manually map out paths or maintain a central routing table!
- **Separation of Concerns**: Isolating shared `middleware` at the root ensures your core app business logic remains pure and modular.

## Schema DSL Reference

### Column Types

| DSL Function | Rust Type | PostgreSQL Type |
|---|---|---|
| `integer("col")` | `i32` | `INTEGER` |
| `short("col")` | `i16` | `SMALLINT` |
| `bigint("col")` | `i64` | `BIGINT` |
| `real("col")` | `f32` | `REAL` |
| `double("col")` | `f64` | `DOUBLE PRECISION` |
| `decimal("col", p, s)` | `BigDecimal` | `DECIMAL(p,s)` |
| `varchar("col", n)` | `String` | `VARCHAR(n)` |
| `text("col")` | `String` | `TEXT` |
| `bool("col")` | `bool` | `BOOLEAN` |
| `date("col")` | `NaiveDate` | `DATE` |
| `time("col")` | `NaiveTime` | `TIME` |
| `datetime("col")` | `NaiveDateTime` | `TIMESTAMP` |
| `datetime("col").tz()` | `DateTime<Utc>` | `TIMESTAMPTZ` |
| `uuid("col")` | `Uuid` | `UUID` |
| `binary("col")` | `Vec<u8>` | `BYTEA` |
| `json("col")` | `serde_json::Value` | `JSON` |
| `jsonb("col")` | `serde_json::Value` | `JSONB` |
| `enumeration("col", T)` | `T` | custom enum |
| `col(Type, "col")` | `Type` | *(custom)* |

### Array Types

| DSL Function | Rust Type | PostgreSQL Type |
|---|---|---|
| `text_array("col")` | `Vec<String>` | `TEXT[]` |
| `int_array("col")` | `Vec<i32>` | `INTEGER[]` |
| `bigint_array("col")` | `Vec<i64>` | `BIGINT[]` |
| `uuid_array("col")` | `Vec<Uuid>` | `UUID[]` |
| `bool_array("col")` | `Vec<bool>` | `BOOLEAN[]` |

### Modifiers

| Modifier | Effect |
|---|---|
| `.primary()` | PRIMARY KEY |
| `.auto_increment()` | Excluded from INSERT (DB-assigned) |
| `.nullable()` | Wraps type in `Option<T>` |
| `.unique()` | UNIQUE constraint |
| `.default("expr")` | DEFAULT expression |
| `.now()` | DEFAULT now() |
| `.tz()` | WITH TIME ZONE (DateTime only) |

### Table Constraints

```rust
model PostTag("post_tags") {
    post_id: integer("post_id"),
    tag_id:  integer("tag_id"),
    @primary_key(post_id, tag_id),  // composite PK
    @unique(post_id, tag_id),       // composite unique
    @index(post_id),                // index
}
```

### Relationships

```rust
model User("users") {
    id: integer("id").primary(),
    posts: array(Post, "author_id"),  // one-to-many
}
```

## Running Examples

The `examples` package contains various isolated runnable examples that demonstrate the framework's features. You can run them using the `cargo run -p examples --bin <name>` syntax.

Here are the available examples:

### Database & ORM Examples
- **Basic CRUD:** `cargo run -p examples --bin basic_crud`
- **Advanced Queries:** `cargo run -p examples --bin advanced_query`
- **Relationships:** `cargo run -p examples --bin relationships`

### API & Framework Examples
- **Schema API:** `cargo run -p examples --bin schema_api`
- **Minimal API:** `cargo run -p examples --bin minimal_api`
- **Macro Test:** `cargo run -p examples --bin test_macro`

## Running Tests

Tests run against unit and integration environments. You can run all of them via:

```bash
cargo test --workspace
```

Or you can test individual crates in isolation if you're working on highly specific feature areas:

- **Core Framework (`floz`):** `cargo test -p floz`
- **Object Relational Mapper (`floz-orm`):** `cargo test -p floz-orm`
- **Macros Compilation (`floz-macros`):** `cargo test -p floz-macros`
- **Command Line Interface (`floz-cli`):** `cargo test -p floz-cli`
- **Terminal Editor (`floz-editor`):** `cargo test -p floz-editor`

## License

MIT
