//! Core persistence constants shared by configuration and schema guards.

pub const CORE_STATE_SLOT: &str = "0.2.0";
pub const SCHEMA_VERSION: i64 = 1;
pub const DEFAULT_CORE_DATABASE_URL: &str = "sqlite://.local/state/core/core.sqlite";

#[path = "persistence/event.rs"]
mod event;
#[path = "persistence/migrations.rs"]
mod migrations;
#[path = "persistence/node.rs"]
mod node;

pub(crate) use event::*;
pub(crate) use node::*;
