use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::app::{AppContext, Context};
use crate::errors::ApiError;

pub type GateFuture = Pin<Box<dyn std::future::Future<Output = bool> + Send + 'static>>;

pub struct ChannelGateEntry {
    pub pattern: &'static str,
    pub handler: fn(ctx: Context, vars: HashMap<String, String>) -> GateFuture,
}

impl ChannelGateEntry {
    pub const fn new(
        pattern: &'static str,
        handler: fn(Context, HashMap<String, String>) -> GateFuture,
    ) -> Self {
        Self { pattern, handler }
    }
}

inventory::collect!(ChannelGateEntry);

/// The central memory bus for Floz Channels
pub struct ChannelBroker {
    pub channels: DashMap<String, broadcast::Sender<String>>,
}

impl ChannelBroker {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
        }
    }

    /// Broadcast a payload to a specific channel
    pub fn broadcast(&self, channel: &str, payload: &str) {
        if let Some(tx) = self.channels.get(channel) {
            // It's perfectly fine if there are no active receivers.
            let _ = tx.send(payload.to_string());
        }
    }
}

impl Default for ChannelBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize, Debug)]
pub struct SubscribeFrame {
    pub action: String, // "subscribe"
    pub channel: String,
}

pub async fn ws_channels_handler(
    _req: ntex::web::HttpRequest,
) -> Result<ntex::web::HttpResponse, ntex::web::Error> {
    Ok(ntex::web::HttpResponse::NotImplemented().finish())
}
