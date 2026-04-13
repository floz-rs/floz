//! Static-dispatch middleware pipeline for floz.
//!
//! Supports both **sync** and **async** middleware in a single pipeline.
//! Uses tuple chaining to build a compile-time middleware stack.
//! Zero-cost: no `dyn`, no boxing, no `async_trait`, fully inlined.
//!
//! # Sync Middleware
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
//! ```
//!
//! # Async Middleware
//! ```ignore
//! use floz::prelude::*;
//!
//! #[derive(Clone)]
//! pub struct RateLimiter { /* redis pool, etc */ }
//!
//! impl AsyncMiddleware for RateLimiter {
//!     async fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
//!         // Async Redis check
//!         None
//!     }
//! }
//! ```
//!
//! # Combined Usage
//! ```ignore
//! App::new()
//!     .server(
//!         ServerConfig::new()
//!             .with_middleware(Cors::permissive())        // sync
//!             .with_middleware(Logger)                    // sync
//!             .with_async_middleware(RateLimiter::new())  // async
//!     )
//!     .run()
//!     .await
//! ```

use ntex::service::{Middleware as NtexMiddleware, Service, ServiceCtx};
use ntex::web::{Error, ErrorRenderer, HttpRequest, HttpResponse, WebRequest, WebResponse};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// The Sync Middleware Trait
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A synchronous floz middleware step.
///
/// Implement this trait for middleware that does NOT require I/O —
/// CORS, tracing, header injection, compression, etc.
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
// The Async Middleware Trait
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// An asynchronous floz middleware step.
///
/// Implement this trait for middleware that requires I/O —
/// database lookups, Redis cache checks, external API calls, etc.
///
/// Uses RPITIT (`impl Future` in traits, stable since Rust 1.75) —
/// no `async_trait` crate, no boxing overhead.
///
/// - Return `None` from `handle()` to continue to the next middleware.
/// - Return `Some(HttpResponse)` to short-circuit.
/// - Override `response()` to post-process asynchronously.
///
/// Must be `Clone` (shared across ntex workers).
///
/// # Note on Send bounds
///
/// ntex uses `Rc`-based internals (`HttpRequest`, `HttpResponse`) which are
/// not `Send`. Since ntex workers are single-threaded, `Send` is not required
/// on the returned futures. This means your async middleware can freely hold
/// references to request/response across `.await` points.
///
/// # Example
/// ```ignore
/// #[derive(Clone)]
/// pub struct JwtAuth { secret: Vec<u8> }
///
/// impl AsyncMiddleware for JwtAuth {
///     async fn handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
///         let token = req.headers().get("Authorization")?;
///         // Async DB lookup to validate token
///         match validate_token_async(token).await {
///             Ok(_claims) => None,  // continue
///             Err(_) => Some(HttpResponse::Unauthorized().finish()),
///         }
///     }
/// }
/// ```
pub trait AsyncMiddleware: Clone + Send + Sync + 'static {
    /// Pre-process the request asynchronously.
    /// Return `None` to continue, `Some(HttpResponse)` to halt.
    fn handle(&self, req: &HttpRequest) -> impl std::future::Future<Output = Option<HttpResponse>>;

    /// Post-process the response asynchronously (runs in reverse middleware order).
    /// Default: pass through unchanged.
    fn response(
        &self,
        _req: &HttpRequest,
        resp: HttpResponse,
    ) -> impl std::future::Future<Output = HttpResponse> {
        async { resp }
    }

    /// Human-readable name for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Layer wrappers — type-level sync/async distinction
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Wraps a sync `Middleware` for use in the pipeline.
///
/// Created automatically by `ServerConfig::with_middleware()`.
/// Users never construct this directly.
#[derive(Clone, Debug)]
pub struct SyncLayer<M: Middleware>(pub M);

/// Wraps an async `AsyncMiddleware` for use in the pipeline.
///
/// Created automatically by `ServerConfig::with_async_middleware()`.
/// Users never construct this directly.
#[derive(Clone, Debug)]
pub struct AsyncLayer<M: AsyncMiddleware>(pub M);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Stack types — tuple chaining
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Empty middleware stack — the base case.
#[derive(Clone, Debug)]
pub struct EmptyStack;

/// A middleware layer wrapping an inner stack.
///
/// Built automatically by `ServerConfig::with_middleware()`
/// and `ServerConfig::with_async_middleware()`.
/// Users never construct this directly.
#[derive(Clone, Debug)]
pub struct Stack<Inner, Outer> {
    pub inner: Inner,
    pub outer: Outer,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Process — recursive async execution
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Trait for executing a middleware stack.
///
/// Implemented recursively on `EmptyStack` and `Stack<I, O>`.
/// The compiler fully inlines the call chain.
///
/// Both sync and async middleware are executed through this unified
/// async interface. For sync middleware, the futures are trivially
/// ready and get optimized away by the compiler.
///
/// # Note on Send
///
/// Futures returned by `Process` methods are NOT required to be `Send`.
/// ntex uses `Rc`-based types internally (`HttpRequest`, `HttpResponse`),
/// and each ntex worker runs on a single thread. This is safe and correct.
pub trait Process: Clone + Send + Sync + 'static {
    /// Run handle() through the stack in insertion order.
    /// Short-circuits on the first `Some(HttpResponse)`.
    fn run_handle(
        &self,
        req: &HttpRequest,
    ) -> impl std::future::Future<Output = Option<HttpResponse>>;

    /// Run response() through the stack in reverse order.
    fn run_response(
        &self,
        req: &HttpRequest,
        resp: HttpResponse,
    ) -> impl std::future::Future<Output = HttpResponse>;
}

impl Process for EmptyStack {
    #[inline]
    async fn run_handle(&self, _req: &HttpRequest) -> Option<HttpResponse> {
        None
    }

    #[inline]
    async fn run_response(&self, _req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        resp
    }
}

// Process for Stack with a sync layer
impl<I: Process, O: Middleware> Process for Stack<I, SyncLayer<O>> {
    #[inline]
    async fn run_handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        // Inner (earlier) middlewares run first
        if let Some(resp) = self.inner.run_handle(req).await {
            return Some(resp);
        }
        // Then this layer (sync — no real await)
        self.outer.0.handle(req)
    }

    #[inline]
    async fn run_response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        // This layer (later middleware) runs first → reverse order
        let resp = self.outer.0.response(req, resp);
        self.inner.run_response(req, resp).await
    }
}

// Process for Stack with an async layer
impl<I: Process, O: AsyncMiddleware> Process for Stack<I, AsyncLayer<O>> {
    #[inline]
    async fn run_handle(&self, req: &HttpRequest) -> Option<HttpResponse> {
        // Inner (earlier) middlewares run first
        if let Some(resp) = self.inner.run_handle(req).await {
            return Some(resp);
        }
        // Then this layer (async)
        self.outer.0.handle(req).await
    }

    #[inline]
    async fn run_response(&self, req: &HttpRequest, resp: HttpResponse) -> HttpResponse {
        // This layer (later middleware) runs first → reverse order
        let resp = self.outer.0.response(req, resp).await;
        self.inner.run_response(req, resp).await
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
        if let Some(resp) = self.middlewares.run_handle(&http_req).await {
            // Halted — still run post-processing
            let resp = self.middlewares.run_response(&http_req, resp).await;
            return Ok(WebResponse::new(resp, http_req));
        }

        // Reassemble and call the handler
        match WebRequest::<Err>::from_parts(http_req, payload) {
            Ok(web_req) => {
                let web_resp = ctx.call(&self.service, web_req).await?;

                // Phase 2: post-processing (reverse order)
                let (resp, req_back) = web_resp.into_parts();
                let resp = self.middlewares.run_response(&req_back, resp).await;
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
