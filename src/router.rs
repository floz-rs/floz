//! Route auto-registration system.
//!
//! Handlers annotated with `#[route(...)]` are automatically collected
//! via `inventory` and registered with ntex's `ServiceConfig` when
//! `App::run()` starts.

/// Response metadata for a route (status code, description, optional content type).
///
/// Created by the `#[route]` proc macro from `resps: [...]`.
type SchemaFn = fn(
    &mut Vec<(String, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>)>
) -> (String, utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>);

#[derive(Clone, Copy)]
pub struct ResponseMeta {
    pub status: u16,
    pub description: &'static str,
    pub content_type: Option<&'static str>,
    pub schema_fn: Option<SchemaFn>,
}

/// A registered route entry, collected via `inventory`.
///
/// These are created by the `#[route]` proc macro — you should
/// never need to construct one manually.
pub struct RouteEntry {
    /// HTTP method as string ("get", "post", "put", "patch", "delete")
    pub method: &'static str,
    /// URL path as written by the user (e.g. "/users/:id")
    pub path: &'static str,
    /// Optional OpenAPI tag for grouping
    pub tag: Option<&'static str>,
    /// Optional description
    pub desc: Option<&'static str>,
    /// Function that registers this handler with ntex's ServiceConfig
    pub register: fn(&mut ntex::web::ServiceConfig),
    /// Response specifications for OpenAPI docs
    pub responses: &'static [ResponseMeta],
    /// Auth requirement: "jwt", "api_key", or "none"
    pub auth: Option<&'static str>,
    /// Rate limit: e.g. "100/min", "10/sec"
    pub rate: Option<&'static str>,
}

impl RouteEntry {
    /// Create a new route entry. Called by the `#[route]` macro codegen.
    pub const fn new(
        method: &'static str,
        path: &'static str,
        tag: Option<&'static str>,
        desc: Option<&'static str>,
        register: fn(&mut ntex::web::ServiceConfig),
        responses: &'static [ResponseMeta],
        auth: Option<&'static str>,
        rate: Option<&'static str>,
    ) -> Self {
        Self { method, path, tag, desc, register, responses, auth, rate }
    }
}

// Tell inventory to collect RouteEntry instances across the binary.
inventory::collect!(RouteEntry);

/// Register all auto-discovered `#[route]` handlers with ntex.
///
/// Called internally by `App::run()`.
pub fn register_all(cfg: &mut ntex::web::ServiceConfig) {
    for entry in inventory::iter::<RouteEntry> {
        (entry.register)(cfg);
    }
}

/// Get all registered routes as a collected Vec.
pub fn all_routes() -> Vec<&'static RouteEntry> {
    inventory::iter::<RouteEntry>.into_iter().collect()
}

/// Print all registered routes to stdout.
///
/// Automatically called in dev mode or when `FLOZ_PRINT_ROUTES=1` is set.
/// Also available for the `floz routes` CLI concept.
pub fn print_route_table() {
    let mut entries: Vec<&RouteEntry> = all_routes();

    if entries.is_empty() {
        println!("  No routes registered.");
        return;
    }

    // Sort by tag (grouped), then by path
    entries.sort_by(|a, b| {
        let tag_cmp = a.tag.unwrap_or("~").cmp(b.tag.unwrap_or("~"));
        if tag_cmp == std::cmp::Ordering::Equal {
            a.path.cmp(b.path)
        } else {
            tag_cmp
        }
    });

    println!();
    println!(
        "  {:<8} {:<28} {:<18} {:<6} {:<10} DESCRIPTION",
        "METHOD", "PATH", "TAG", "AUTH", "RATE"
    );
    println!(
        "  {:<8} {:<28} {:<18} {:<6} {:<10} ───────────────────────────────",
        "──────", "────────────────────────────",
        "──────────────────", "──────", "──────────"
    );

    for entry in &entries {
        let auth_display = entry.auth.unwrap_or("—");
        let rate_display = entry.rate.unwrap_or("—");

        println!(
            "  {:<8} {:<28} {:<18} {:<6} {:<10} {}",
            entry.method.to_uppercase(),
            entry.path,
            entry.tag.unwrap_or("—"),
            auth_display,
            rate_display,
            entry.desc.unwrap_or(""),
        );
    }
    println!();
}

/// Generate the utoipa OpenAPI spec dynamically at runtime
/// from all collected `#[route]` handlers.
pub fn generate_openapi() -> utoipa::openapi::OpenApi {
    use utoipa::openapi::{
        path::{HttpMethod, OperationBuilder, PathItem},
        InfoBuilder, OpenApiBuilder, PathsBuilder, ResponseBuilder, ResponsesBuilder,
        ComponentsBuilder, ContentBuilder,
    };

    let mut paths = PathsBuilder::new();
    let mut components = ComponentsBuilder::new();

    for entry in all_routes() {
        // 1. Map HTTP Method
        let method = match entry.method {
            "get" => HttpMethod::Get,
            "post" => HttpMethod::Post,
            "put" => HttpMethod::Put,
            "patch" => HttpMethod::Patch,
            "delete" => HttpMethod::Delete,
            _ => continue,
        };

        // 2. Map Responses
        let mut responses = ResponsesBuilder::new();
        for r in entry.responses {
            let mut resp = ResponseBuilder::new().description(r.description);

            // If a custom schema was provided via `Json<Type>` or similar:
            if let Some(schema_fn) = r.schema_fn {
                // Execute the function pointer dynamically!
                let mut components_list = Vec::new();
                let (root_name, root_schema) = schema_fn(&mut components_list);
                
                // Construct the JSON content referring to this root schema component
                let ref_path = format!("#/components/schemas/{}", root_name);
                let content = ContentBuilder::new()
                    .schema(Some(utoipa::openapi::schema::Ref::new(ref_path)))
                    .build();

                // Bind to application/json as default mapping
                resp = resp.content("application/json", content);

                // Register root schema
                components = components.schema(root_name, root_schema);

                // Register all nested exported schemas deeply into OpenAPI components payload
                for (name, schema) in components_list {
                    components = components.schema(name, schema);
                }
            }

            responses = responses.response(r.status.to_string(), resp.build());
        }

        // 3. Build Operation
        let op_id = entry.path.replace(['/', ':'], "_");
        let mut op = OperationBuilder::new()
            .operation_id(Some(op_id))
            .responses(responses);

        if let Some(desc) = entry.desc {
            op = op.description(Some(desc));
        }

        if let Some(tag) = entry.tag {
            op = op.tag(tag);
        }

        // Handle path parameters for openapi: transform :id to {id}
        // Actually, ntex already expects `{id}` if we translate it in the macro.
        // But the entry.path from the macro is the original `:id`.
        let openapi_path = entry.path.to_string();
        
        let path_item = PathItem::new(method, op.build());
        
        // Transform Express-style `/users/:id` to OpenAPI `/users/{id}`
        // and also we could add parameters here, but let's just do path translation for now.
        let mut converted_path = String::with_capacity(openapi_path.len());
        let mut chars = openapi_path.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == ':' {
                converted_path.push('{');
                while let Some(&next) = chars.peek() {
                    if next == '/' || next == '.' || next == '-' {
                        break;
                    }
                    converted_path.push(chars.next().unwrap());
                }
                converted_path.push('}');
            } else {
                converted_path.push(ch);
            }
        }

        paths = paths.path(converted_path, path_item);
    }

    OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title("Floz API")
                .description(Some("Auto-generated OpenAPI docs"))
                .version("1.0.0")
                .build(),
        )
        .paths(paths)
        .components(Some(components.build()))
        .build()
}

/// A simple, standalone Swagger UI HTML page that pulls the OpenAPI spec from `/api-docs/openapi.json`.
pub const SWAGGER_UI_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Floz API Docs</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
    <style>
      body { margin: 0; box-sizing: border-box; }
      #swagger-ui { max-width: 1200px; margin: 0 auto; }
    </style>
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js" crossorigin></script>
    <script>
      window.onload = () => {
        window.ui = SwaggerUIBundle({
          url: '/api-docs/openapi.json',
          dom_id: '#swagger-ui',
        });
      };
    </script>
  </body>
</html>
"#;
