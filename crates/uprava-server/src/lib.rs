//! Uprava Core composition boundary.
//!
//! Configuration, domain rules, persistence, observability and the HTTP/control
//! application are composed behind this narrow public API.

mod runtime;

pub use runtime::{build_router, shutdown_signal, AppConfig, AppError, AppState, ConfigError};
