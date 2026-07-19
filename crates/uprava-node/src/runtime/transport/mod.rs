//! Outbound Core HTTP and WebSocket adapters.

mod control;
mod enrollment;

pub(crate) use control::*;
pub(crate) use enrollment::*;
