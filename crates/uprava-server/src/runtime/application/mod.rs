//! Core use-case orchestration and read-model assembly.

mod coordination;
mod projection;
mod scheduling;
mod session;
mod workspace;

pub(crate) use coordination::*;
pub(crate) use projection::*;
pub(crate) use scheduling::*;
pub(crate) use session::*;
pub(crate) use workspace::*;
