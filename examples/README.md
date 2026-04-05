# floz Examples

Working examples demonstrating the different layers of the `floz` framework.

> **Prerequisites:** A running PostgreSQL database. By default, examples connect to
> `postgres://localhost:5432/floz1`. Override with the `DATABASE_URL` environment variable.

## Directory Structure

```text
src/bin/
├── sqlx/                   # Direct database examples (manual SQL / query construction)
│   ├── basic_crud.rs       # Creating tables, custom DAO objects loosely coupled to DB wrapper
│   ├── advanced_query.rs   # Using the DSL builder and converting to manual SQL queries
│   └── relationships.rs    # Manual management of related records
│
├── orm/                    # Pure floz-orm examples (ActiveRecord and true DSL)
│   ├── dao.rs              # The DAO API: User::create(), save(), delete() with dirty tracking
│   └── dsl.rs              # The DSL API: UserTable::select().where_().execute()
│
└── api/                    # HTTP / API examples (floz framework routing)
    ├── minimal_api.rs      # Minimal route registration
    └── schema_api.rs       # OpenAPI schema generation with utoipa
```

## Running Examples

Execute any example by name with `cargo run`:

```sh
DATABASE_URL=postgres://localhost:5432/mydb cargo run -p examples --bin <name>
```

### Raw SQLx (Direct Query) Examples

| Example | Command | What it demonstrates |
|---------|---------|---------------------|
| Basic CRUD | `cargo run -p examples --bin sqlx_basic_crud` | Direct SQL execution with `db.execute_raw` and custom structs |
| Advanced Query | `cargo run -p examples --bin sqlx_advanced_query` | Compiling a query builder manually via `.to_sql()` and executing it |
| Relationships | `cargo run -p examples --bin sqlx_relationships` | Running relationship queries with explicit join strings/SQL |

### Pure ORM Examples

| Example | Command | What it demonstrates |
|---------|---------|---------------------|
| DAO API | `cargo run -p examples --bin orm_dao` | `User::create().execute(&db)`, `.save()`, `.delete()` (Active Record) |
| DSL API | `cargo run -p examples --bin orm_dsl` | `Table::select()`, `update()`, `insert_many()` (Query Builder) |

### API Examples

| Example | Command | What it demonstrates |
|---------|---------|---------------------|
| Minimal API | `cargo run -p examples --bin api_minimal` | Route macro, minimal server setup |
| Schema API | `cargo run -p examples --bin api_schema` | OpenAPI schema, typed responses, middleware |
