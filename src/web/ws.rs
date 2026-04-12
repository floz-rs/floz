//! WebSocket abstraction for floz.
//!
//! Exposes native WebSockets via Ntex.
//!
//! ```ignore
//! use floz::prelude::*;
//! 
//! #[route(get: "/ws/chat")]
//! async fn chat_ws(req: HttpRequest) -> Result<HttpResponse, Error> {
//!     web::ws::start(req, None::<&str>, |sink| async move {
//!         // Create a websocket handler service logic here...
//!         todo!()
//!     }).await
//! }
//! ```

pub use ntex::web::ws::*;
