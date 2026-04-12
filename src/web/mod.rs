pub use ntex::web::{
    self,
    middleware,
    types::{Path, Json, Query, Payload, State},
    Error, HttpRequest, HttpResponse, HttpResponseBuilder
};
pub use ntex::http::{header, StatusCode};

#[cfg(feature = "ws")]
pub mod ws;

pub mod upload;
pub mod channels;
