//! Static-dispatch middleware pipeline for floz.
//!
//! Uses tuple chaining to build a compile-time middleware stack.
//! Zero-cost: no `dyn`, no boxing, no `async_trait`, fully inlined.
//!
//! # Example
//! ```ignore
//! use floz::prelude::*;
//!
//! #[derive(Clone)]
//! pub struct Logger;
//!
//! impl Middleware for Logger {
//!     fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
//!         tracing::info!("→ {} {}", req.method(), req.path());
//!         None // continue
//!     }
//!
//!     fn response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
//!         tracing::info!("← {}", resp.status());
//!         resp
//!     }
//! }
//!
//! // Usage:
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(Cors::permissive())
//!             .with_middleware(Logger)
//!     )
//!     .run()
//!     .await
//! ```

use ntex::service::{Middleware as NtexMiddleware, Service, ServiceCtx};
use ntex::web::{Error, ErrorRenderer, HttpRequest, HttpResponse, WebRequest, WebResponse};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// The Middleware Trait
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A floz middleware step.
///
/// Implement this trait to add cross-cutting concerns (auth, logging,
/// rate limiting, CORS, etc.) to the request pipeline.
///
/// - Return `None` from `handle()` to continue to the next middleware.
/// - Return `Some(HttpResponse)` to short-circuit (auth fail, etc.).
/// - Override `response()` to post-process (add headers, log timing).
///
/// Must be `Clone` (shared across ntex workers).
///
/// # Early exit
/// ```ignore
/// fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
///     Some(HttpResponse::Unauthorized().finish())
/// }
/// ```
///
/// # Post-processing
/// ```ignore
/// fn response(&self, _req: &HttpRequest, mut resp: HttpResponse) -> HttpResponse {
///     resp.headers_mut().insert("X-Custom", "value".parse().unwrap());
///     resp
/// }
/// ```
pub trait Middleware: Clone + Send + Sync + 'static {
    /// Pre-process the request.
    /// Return `None` to continue, `Some(HttpResponse)` to halt.
    fn handle(&self, req: &HttpRequest) -> Option<HttpResponse>;

    /// Post-process the response (runs in reverse middleware order).
    /// Default: pass through unchanged.
    fn response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        resp
    }

    /// Human-readable name for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Stack types — tuple chaining
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Empty middleware stack — the base case.
#[derive(Clone, Debug)]
pub struct EmptyStack;

/// A middleware layer wrapping an inner stack.
///
/// Built automatically by `ServerConfig::with_middleware()`.
/// Users never construct this directly.
#[derive(Clone, Debug)]
pub struct Stack<Inner, Outer> {
    pub inner: Inner,
    pub outer: Outer,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Process — recursive execution
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Trait for executing a middleware stack.
///
/// Implemented recursively on `EmptyStack` and `Stack<I, O>`.
/// The compiler fully inlines the call chain.
pub trait Process: Clone + Send + Sync + 'static {
    /// Run handle() through the stack in insertion order.
    /// Short-circuits on the first `Some(HttpResponse)`.
    fn run_handle(&self, req: &HttpRequest) -> Option<HttpResponse>;

    /// Run response() through the stack in reverse order.
    fn run_response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse;
}

impl Process for EmptyStack {
    #[inline]
    fn run_handle(&self, _req: &HttpRequest) -> Option<HttpResponse> {
        None
    }

    #[inline]
    fn run_response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        resp
    }
}

impl<I: Process, O: Middleware> Process for Stack<I, O> {
    #[inline]
    fn run_handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        // Inner (earlier) middlewares run first
        if let Some(resp) = self.inner.run_handle(req) {
            return Some(resp);
        }
        // Then this layer
        self.outer.handle(req)
    }

    #[inline]
    fn run_response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        // This layer (later middleware) runs first → reverse order
        let resp = self.outer.response(req, resp);
        self.inner.run_response(req, resp)
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FlozPipeline — the ntex bridge
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Single ntex middleware wrapping the entire floz pipeline.
///
/// Fully monomorphized — the compiler inlines the entire stack.
/// Zero overhead compared to hand-written ntex middleware.
#[derive(Clone)]
pub struct FlozPipeline<M: Process> {
    pub(crate) middlewares: M,
}

impl<M: Process> FlozPipeline<M> {
    pub fn new(middlewares: M) -> Self {
        Self { middlewares }
    }
}

impl<S, C, M: Process> NtexMiddleware<S, C> for FlozPipeline<M> {
    type Service = FlozPipelineService<S, M>;

    fn create(&self, service: S, _cfg: C) -> Self::Service {
        FlozPipelineService {
            service,
            middlewares: self.middlewares.clone(),
        }
    }
}

/// The ntex service created by `FlozPipeline`.
pub struct FlozPipelineService<S, M> {
    service: S,
    middlewares: M,
}

impl<S, M, Err> Service<WebRequest<Err>> for FlozPipelineService<S, M>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
    M: Process,
    Err: ErrorRenderer,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_ready!(service);
    ntex::forward_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        // Split into HttpRequest + Payload
        let (http_req, payload) = req.into_parts();

        // Phase 1: pre-processing (insertion order)
        if let Some(resp) = self.middlewares.run_handle(&http_req) {
            // Halted — still run post-processing
            let resp = self.middlewares.run_response(&http_req, resp);
            return Ok(WebResponse::new(resp, http_req));
        }

        // Reassemble and call the handler
        match WebRequest::<Err>::from_parts(http_req, payload) {
            Ok(web_req) => {
                let web_resp = ctx.call(&self.service, web_req).await?;

                // Phase 2: post-processing (reverse order)
                let (resp, req_back) = web_resp.into_parts();
                let resp = self.middlewares.run_response(&req_back, resp);
                Ok(WebResponse::new(resp, req_back))
            }
            Err((http_req, _)) => {
                let resp = HttpResponse::InternalServerError()
                    .body("Pipeline error: request Rc was cloned by middleware");
                Ok(WebResponse::new(resp, http_req))
            }
        }
    }
}
