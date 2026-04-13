//! File upload abstraction for floz.
//!
//! Provides ergonomic file extraction and natively embeds `ntex_multipart`.
//!
//! ```ignore
//! use floz::prelude::*;
//! use floz::web::upload;
//!
//! #[route(post: "/upload")]
//! async fn handle_upload(mut payload: upload::multipart::Multipart) -> Result<HttpResponse, Error> {
//!     while let Some(item) = payload.next().await {
//!         let field = item?;
//!         upload::save_field(field, "/tmp/uploaded_file.bin").await?;
//!     }
//!     Ok(HttpResponse::Ok().finish())
//! }
//! ```

pub use ntex_multipart as multipart;

use futures::StreamExt;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Saves a multipart web stream directly to the underlying disk asynchronously.
/// This prevents memory bloat by chunking the payload exactly as it's streamed inside.
pub async fn save_field<P: AsRef<Path>>(
    mut field: multipart::Field,
    path: P,
) -> std::io::Result<()> {
    let mut file = File::create(path).await?;
    while let Some(chunk) = field.next().await {
        let bytes = chunk
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        file.write_all(&bytes).await?;
    }
    file.flush().await?;
    Ok(())
}
