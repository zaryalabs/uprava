//! Uprava Node daemon library boundary.
//!
//! The binary is intentionally a thin composition root. Runtime ownership,
//! persistence, control transport, workspace and PTY behavior live behind
//! [`run`].

mod runtime;

pub use runtime::run;
