//! Middleware for floz — CORS, compression, tracing, and more.
//!
//! Static-dispatch pipeline using tuple chaining.
//! Zero overhead — fully monomorphized at compile time.

pub mod cors;
pub mod pipeline;
pub mod trace;

#[cfg(feature = "compression")]
pub mod compression;

// Re-export core types
pub use pipeline::{Middleware, Process, EmptyStack, Stack, FlozPipeline};
