use crate::app::AppContext;
use ntex::web::{Error, FromRequest, HttpRequest};
use std::future::{ready, Ready};
use std::sync::Arc;

/// The Unified Context Extractor for Floz handlers.
///
/// This provides a perfect separation of concerns:
/// - `ctx.app`: The global, read-only configuration and connection pools (Fixed Context).
/// - `ctx.req`: The isolated, per-request session and authentication variables (Shared Context).
pub struct Context {
    /// The global application context (database pools, cache, environment config)
    pub app: Arc<AppContext>,
    /// The specific context for the current executing HTTP request
    pub req: RequestContext,
}

impl Context {
    /// Ergonomic accessor for the session store
    pub fn session(&self) -> crate::session::SessionStore<'_> {
        self.req.session(&self.app)
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AuthInfo {
    pub user_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl AuthInfo {
    pub fn is_authenticated(&self) -> bool {
        self.user_id.is_some()
    }
    
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == perm)
    }
}

/// The isolated context for the current HTTP request.
#[derive(Clone)]
pub struct RequestContext {
    /// The session ID uniquely resolving the current visitor
    pub session_id: String,
    
    /// Authentication context representing the currently authenticated identity
    pub auth: AuthInfo,
}

impl RequestContext {
    /// Gain access to the isolated Redis SessionStore for the current user.
    pub fn session<'a>(&'a self, app: &'a AppContext) -> crate::session::SessionStore<'a> {
        #[cfg(feature = "worker")]
        {
            crate::session::SessionStore::new(self.session_id.clone(), app.cache.as_ref())
        }
        #[cfg(not(feature = "worker"))]
        {
            crate::session::SessionStore::new(self.session_id.clone())
        }
    }
}

impl<Err: ntex::web::ErrorRenderer> FromRequest<Err> for Context {
    type Error = Error;

    #[inline]
    async fn from_request(req: &HttpRequest, _: &mut ntex::http::Payload) -> Result<Self, Self::Error> {
        // 1. Extract the Fixed AppContext
        let app = req.app_state::<AppContext>()
            .expect("AppContext must be initialized centrally via App::new().state()")
            .clone();

        // 2. Extract the RequestContext
        let req_ctx = req.extensions()
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_else(|| RequestContext {
                // If the middleware hasn't run yet or isn't placed, use a stub.
                session_id: "anonymous_stub".to_string(),
                auth: AuthInfo::default(),
            });

        let app_arc = Arc::new(app);

        Ok(Context {
            app: app_arc,
            req: req_ctx,
        })
    }
}
