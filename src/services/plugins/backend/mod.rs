//! Plugin backend abstraction layer
//!
//! This module provides the JavaScript runtime backend for executing TypeScript plugins.
//! Currently implements QuickJS with oxc transpilation.

pub mod quickjs_backend;

pub use quickjs_backend::{PendingResponses, QuickJsBackend, TsPluginInfo};
