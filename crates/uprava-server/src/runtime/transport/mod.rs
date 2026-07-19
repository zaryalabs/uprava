//! Axum and control-channel adapters for Core application use cases.

mod http;
mod live;
mod mcp;
mod node;

pub(crate) use http::*;
pub use http::{build_router, shutdown_signal};
pub(crate) use live::*;
pub(crate) use mcp::*;
pub(crate) use node::*;
