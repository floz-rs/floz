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
    /// Authorization requirement (e.g. "required", "optional", "none")
    pub auth: Option<&'static str>,
    /// Array of string permissions required to access the route
    pub permissions: Option<&'static [&'static str]>,
    /// Rate limit: e.g. "100/min", "10/sec"
    pub rate: Option<&'static str>,
    /// Request schema function (optional)
    pub req_body_schema_fn: Option<SchemaFn>,
    /// Whether this endpoint supports standard PaginationParams (adds query args to OpenAPI docs)
    pub is_paginated: bool,
    /// Whether this endpoint accepts the ?preload query parameter
    pub has_preload: bool,
    /// Optional cache TTL in seconds 
    pub cache_ttl: Option<u64>,
    /// Optional list of table-dependent tags to auto-invalidate this cache
    pub cache_watch: Option<&'static [&'static str]>,
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
        permissions: Option<&'static [&'static str]>,
        rate: Option<&'static str>,
        req_body_schema_fn: Option<SchemaFn>,
        is_paginated: bool,
        has_preload: bool,
        cache_ttl: Option<u64>,
        cache_watch: Option<&'static [&'static str]>,
    ) -> Self {
        Self { method, path, tag, desc, register, responses, auth, permissions, rate, req_body_schema_fn, is_paginated, has_preload, cache_ttl, cache_watch }
    }
}

// Tell inventory to collect RouteEntry instances across the binary.
inventory::collect!(RouteEntry);

/// Translate `:param` style path segments to `{param}` for ntex pattern matching.
fn translate_path(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ':' {
            result.push('{');
            while let Some(&next) = chars.peek() {
                if next == '/' || next == '.' || next == '-' {
                    break;
                }
                result.push(chars.next().unwrap());
            }
            result.push('}');
        } else {
            result.push(ch);
        }
    }
    result
}

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

/// Lightweight cache metadata extracted from a `RouteEntry`.
///
/// Stored in a shared `HashMap` and looked up by the `CacheMiddleware`
/// on every request to determine if/how to cache the response.
#[derive(Clone, Debug)]
pub struct CacheRouteInfo {
    /// Original path pattern with `:param` syntax (e.g. "/users/:id")
    pub path_pattern: &'static str,
    /// Cache TTL in seconds
    pub ttl: u64,
    /// Table dependency tags for invalidation (e.g. ["users", "users:{id}"])
    pub watch: Vec<&'static str>,
}

/// Build a lookup map of `"METHOD /ntex/path/{pattern}"` → `CacheRouteInfo`
/// for all routes that have `cache_ttl` configured.
///
/// Called once during `App::run()` and injected into `AppContext`.
pub fn build_cache_route_map() -> std::collections::HashMap<String, CacheRouteInfo> {
    let mut map = std::collections::HashMap::new();
    for entry in inventory::iter::<RouteEntry> {
        if let Some(ttl) = entry.cache_ttl {
            let ntex_path = translate_path(entry.path);
            let key = format!("{} {}", entry.method.to_uppercase(), ntex_path);
            let watch = entry.cache_watch
                .map(|tags| tags.to_vec())
                .unwrap_or_default();
            map.insert(key, CacheRouteInfo {
                path_pattern: entry.path,
                ttl,
                watch,
            });
        }
    }
    map
}

/// Security metadata extracted from a `RouteEntry`.
///
/// Looked up by `AuthMiddleware` to enforce route protections.
#[derive(Clone, Debug)]
pub struct RouteSecurityRule {
    /// E.g. "jwt", "api_key", or None (anonymous)
    pub auth: Option<&'static str>,
    /// Any required permissions strings
    pub permissions: Vec<&'static str>,
}

/// Type alias for the shared security route map injected into ntex app state.
pub type SecurityRouteMap = std::sync::Arc<std::collections::HashMap<String, RouteSecurityRule>>;

/// Build a lookup map of `"METHOD /ntex/path/{pattern}"` → `RouteSecurityRule`
/// for all routes that have `auth` or `permissions` configured.
///
/// Called once during `App::run()` and injected into the ntex app state.
pub fn build_security_route_map() -> std::collections::HashMap<String, RouteSecurityRule> {
    let mut map = std::collections::HashMap::new();
    for entry in inventory::iter::<RouteEntry> {
        if entry.auth.is_some() || entry.permissions.is_some() {
            let ntex_path = translate_path(entry.path);
            let key = format!("{} {}", entry.method.to_uppercase(), ntex_path);
            let permissions = entry.permissions
                .map(|p| p.to_vec())
                .unwrap_or_default();
            map.insert(key, RouteSecurityRule {
                auth: entry.auth,
                permissions,
            });
        }
    }
    map
}

/// Type alias for the shared rate limit route map injected into ntex app state.
pub type RateLimitRouteMap = std::sync::Arc<std::collections::HashMap<String, String>>;

/// Build a lookup map of `"METHOD /ntex/path/{pattern}"` → `String` (rate limit string)
/// for all routes that have `rate` configured. Alternatively falls back to the global
/// rate limit if defined.
///
/// Called once during `App::run()` and injected into the ntex app state.
pub fn build_rate_limit_route_map(global_rate_limit: Option<String>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for entry in inventory::iter::<RouteEntry> {
        let ntex_path = translate_path(entry.path);
        let key = format!("{} {}", entry.method.to_uppercase(), ntex_path);
        
        if let Some(rate) = entry.rate {
            map.insert(key, rate.to_string());
        } else if let Some(global) = &global_rate_limit {
            map.insert(key, global.clone());
        }
    }
    map
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
                
                let content = if root_name.is_empty() {
                    ContentBuilder::new()
                        .schema(Some(root_schema))
                        .build()
                } else {
                    let ref_path = format!("#/components/schemas/{}", root_name);
                    components = components.schema(root_name, root_schema);
                    ContentBuilder::new()
                        .schema(Some(utoipa::openapi::schema::Ref::new(ref_path)))
                        .build()
                };

                // Bind to application/json as default mapping
                resp = resp.content("application/json", content);

                // Register all nested exported schemas deeply into OpenAPI components payload
                for (name, schema) in components_list {
                    components = components.schema(name, schema);
                }
            }

            responses = responses.response(r.status.to_string(), resp.build());
        }

        // 3. Build Operation
        let clean_path = entry.path.replace(['/', ':', '{', '}'], "_");
        let op_id = format!("{}_{}", entry.method, clean_path);
        let mut op = OperationBuilder::new()
            .operation_id(Some(op_id))
            .responses(responses);

        if let Some(schema_fn) = entry.req_body_schema_fn {
            let mut components_list = Vec::new();
            let (root_name, root_schema) = schema_fn(&mut components_list);
            
            let content = if root_name.is_empty() {
                ContentBuilder::new()
                    .schema(Some(root_schema))
                    .build()
            } else {
                let ref_path = format!("#/components/schemas/{}", root_name);
                components = components.schema(root_name, root_schema);
                ContentBuilder::new()
                    .schema(Some(utoipa::openapi::schema::Ref::new(ref_path)))
                    .build()
            };
                
            let request_body = utoipa::openapi::request_body::RequestBodyBuilder::new()
                .content("application/json", content)
                .required(Some(utoipa::openapi::Required::True))
                .build();
                
            op = op.request_body(Some(request_body));
            
            for (name, schema) in components_list {
                components = components.schema(name, schema);
            }
        }

        if let Some(desc) = entry.desc {
            op = op.description(Some(desc));
        }

        if let Some(tag) = entry.tag {
            op = op.tag(tag);
        }

        if entry.is_paginated {
            use ::utoipa::openapi::path::{ParameterBuilder, ParameterIn};
            use ::utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
            
            op = op.parameter(
                ParameterBuilder::new()
                    .name("limit")
                    .parameter_in(ParameterIn::Query)
                    .description(Some("Maximum number of results to return (default 10)"))
                    .schema(Some(ObjectBuilder::new().schema_type(Type::Integer).build()))
                    .build()
            );
            op = op.parameter(
                ParameterBuilder::new()
                    .name("offset")
                    .parameter_in(ParameterIn::Query)
                    .description(Some("Number of results to skip (default 0)"))
                    .schema(Some(ObjectBuilder::new().schema_type(Type::Integer).build()))
                    .build()
            );
            op = op.parameter(
                ParameterBuilder::new()
                    .name("order_by")
                    .parameter_in(ParameterIn::Query)
                    .description(Some("Order direction, e.g., 'created_at -desc'"))
                    .schema(Some(ObjectBuilder::new().schema_type(Type::String).build()))
                    .build()
            );
        }

        if entry.has_preload {
            use ::utoipa::openapi::path::{ParameterBuilder, ParameterIn};
            use ::utoipa::openapi::schema::{ObjectBuilder, Type};
            
            op = op.parameter(
                ParameterBuilder::new()
                    .name("preload")
                    .parameter_in(ParameterIn::Query)
                    .description(Some("Comma-separated list of relationships to eager load (e.g., 'user_roles,posts')"))
                    .schema(Some(ObjectBuilder::new().schema_type(Type::String).build()))
                    .build()
            );
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

/// Bundled Swagger UI JavaScript bundle (embedded at compile time).
pub const SWAGGER_UI_BUNDLE_JS: &str = include_str!("swagger-ui-dist/swagger-ui-bundle.js");

/// Bundled Swagger UI CSS (embedded at compile time).
pub const SWAGGER_UI_CSS: &str = include_str!("swagger-ui-dist/swagger-ui.css");

/// A simple, standalone Swagger UI HTML page that loads assets from local routes.
///
/// All JavaScript and CSS are served from `/api-docs/` paths — no external CDN dependencies.
pub const SWAGGER_UI_HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Floz API Docs</title>
    <link rel="stylesheet" href="/api-docs/swagger-ui.css" />
    <style>
      body { margin: 0; box-sizing: border-box; transition: background-color 0.3s; }
      #swagger-ui { max-width: 1200px; margin: 0 auto; }
    </style>
    <style id="dark-theme-style" media="not all">
{DARK_THEME_CSS}
    </style>
  </head>
  <body>
    <!-- Theme Toggler -->
    <div style="position: absolute; top: 15px; right: 20px; z-index: 1000; font-family: sans-serif;">
      <select id="theme-select" style="padding: 6px; border-radius: 4px; background: #2d2d2d; color: #b3b3b3; border: 1px solid #404040; cursor: pointer;">
        <option value="system">Auto (System)</option>
        <option value="light">Day</option>
        <option value="dark">Night</option>
      </select>
    </div>

    <div id="swagger-ui"></div>
    <script src="/api-docs/swagger-ui-bundle.js"></script>
    <script>
      function applyTheme(theme) {
        let isDark = false;
        if (theme === 'dark') {
          isDark = true;
        } else if (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
          isDark = true;
        }

        const darkStyle = document.getElementById('dark-theme-style');
        if (isDark) {
          darkStyle.media = 'all';
          document.body.style.backgroundColor = '#1f1f1f';
        } else {
          darkStyle.media = 'not all';
          document.body.style.backgroundColor = '#ffffff';
        }
        localStorage.setItem('swagger-theme-pref', theme);
      }

      window.onload = () => {
        window.ui = SwaggerUIBundle({
          url: '/api-docs/openapi.json',
          dom_id: '#swagger-ui',
        });

        // Initialize Theme
        const themeSelect = document.getElementById('theme-select');
        const savedTheme = localStorage.getItem('swagger-theme-pref') || 'system';
        themeSelect.value = savedTheme;
        applyTheme(savedTheme);

        // Listen for user changes
        themeSelect.addEventListener('change', (e) => {
          applyTheme(e.target.value);
        });

        // Listen for system preference changes if 'system' is selected
        window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {
          if (themeSelect.value === 'system') {
            applyTheme('system');
          }
        });
      };
    </script>
  </body>
</html>
"#;

