//! JavaScript Runtime Backend Abstraction
//!
//! This module provides an abstraction layer for JavaScript runtime backends.
//!
//! # Available Backends
//!
//! - **QuickJS** (default): Lightweight embedded JS engine (~700KB) with oxc for
//!   TypeScript transpilation. Best for smaller binary size.
//!
//! - **deno_core**: Embedded V8 engine with deno_ast for TypeScript. Larger but
//!   more compatible with modern JavaScript features.
//!
//! # Feature Flags
//!
//! - `js-quickjs` (default): Use QuickJS + oxc backend
//! - `js-deno-core`: Use deno_core + V8 backend

use crate::services::plugins::api::{EditorStateSnapshot, PluginCommand, PluginResponse};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Information about a loaded TypeScript plugin
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin file path
    pub path: PathBuf,
    /// Whether the plugin is enabled
    pub enabled: bool,
}

// QuickJS backend (default)
#[cfg(feature = "js-quickjs")]
pub mod quickjs_backend;

// deno_core backend (optional)
#[cfg(feature = "js-deno-core")]
pub mod deno_core_backend;

/// Pending response senders type alias for convenience
pub type PendingResponses =
    Arc<std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<PluginResponse>>>>;

/// JavaScript Runtime Backend Trait
///
/// This trait abstracts the JavaScript runtime, allowing different backends
/// to be used interchangeably.
///
/// Note: This trait does NOT require `Send` because JavaScript runtimes
/// (like V8 and QuickJS) are typically not thread-safe. The runtime is
/// designed to run on a dedicated plugin thread.
#[allow(async_fn_in_trait)]
pub trait JsBackend {
    /// Create a new backend instance with the given configuration
    fn new(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: std::sync::mpsc::Sender<PluginCommand>,
        pending_responses: PendingResponses,
    ) -> Result<Self>
    where
        Self: Sized;

    /// Load and execute a TypeScript/JavaScript module file
    async fn load_module(&mut self, path: &str, plugin_source: &str) -> Result<()>;

    /// Execute a global function by name (for plugin actions)
    async fn execute_action(&mut self, action_name: &str) -> Result<()>;

    /// Emit an event to all registered handlers
    ///
    /// Returns `Ok(true)` if all handlers returned true, `Ok(false)` if any returned false.
    async fn emit(&mut self, event_name: &str, event_data: &str) -> Result<bool>;

    /// Check if any handlers are registered for an event
    fn has_handlers(&self, event_name: &str) -> bool;

    /// Deliver a response to a pending async operation
    fn deliver_response(&self, response: PluginResponse);

    /// Send a status message to the editor UI
    fn send_status(&mut self, message: String);

    /// Get the pending responses handle
    fn pending_responses(&self) -> &PendingResponses;
}

// Re-export the selected backend type based on feature flag
#[cfg(feature = "js-quickjs")]
pub use quickjs_backend::QuickJsBackend;

#[cfg(feature = "js-deno-core")]
pub use deno_core_backend::DenoCoreBackend;

/// The selected backend type
#[cfg(feature = "js-quickjs")]
pub type SelectedBackend = QuickJsBackend;

#[cfg(all(feature = "js-deno-core", not(feature = "js-quickjs")))]
pub type SelectedBackend = DenoCoreBackend;

/// Get the name of the current JS backend
#[cfg(feature = "js-quickjs")]
pub fn backend_name() -> &'static str {
    "QuickJS + oxc"
}

#[cfg(all(feature = "js-deno-core", not(feature = "js-quickjs")))]
pub fn backend_name() -> &'static str {
    "deno_core (embedded V8)"
}

/// Check if the selected runtime is available on the system
pub fn check_runtime_available() -> Result<()> {
    // Both backends are embedded, always available
    Ok(())
}

/// Create a new backend instance
pub fn create_backend(
    state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
    command_sender: std::sync::mpsc::Sender<PluginCommand>,
    pending_responses: PendingResponses,
) -> Result<SelectedBackend> {
    SelectedBackend::new(state_snapshot, command_sender, pending_responses)
}
