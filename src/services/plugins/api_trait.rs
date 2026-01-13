//! Plugin API trait definition
//!
//! This module defines the `EditorApi` trait that specifies all methods
//! available to TypeScript plugins. The QuickJsBackend implements this trait,
//! and the compiler will catch any signature mismatches.
//!
//! ## Method Naming Convention
//!
//! - Sync methods: Regular method names (e.g., `getActiveBufferId`)
//! - Async methods: End with `Start` and return callback_id (e.g., `delayStart`)
//!   The JS runtime wraps these in Promises.
//!
//! ## Adding New API Methods
//!
//! 1. Add the method signature to this trait
//! 2. Implement the method in `QuickJsBackend`
//! 3. Add the JS binding in `setup_global_api()`
//! 4. Update fresh.d.ts if needed
//!
//! The compiler will error if the implementation doesn't match the trait.

use std::sync::mpsc::Sender;
use crate::services::plugins::api::PluginCommand;

/// The Editor API trait
///
/// All methods available to TypeScript plugins are defined here.
/// QuickJsBackend implements this trait for compile-time signature checking.
///
/// Methods are grouped by category for easier navigation.
pub trait EditorApi {
    // ========================================
    // Status and Logging
    // ========================================

    /// Display a message in the status bar
    fn set_status(&self, message: &str);

    /// Log a debug message
    fn log_debug(&self, message: &str);

    /// Log an error message
    fn log_error(&self, message: &str);

    /// Log a warning message
    fn log_warn(&self, message: &str);

    /// Log an info message
    fn log_info(&self, message: &str);

    // ========================================
    // Buffer Queries (from snapshot)
    // ========================================

    /// Get the active buffer ID
    fn get_active_buffer_id(&self) -> u32;

    /// Get the active split ID
    fn get_active_split_id(&self) -> u32;

    /// Get cursor byte position in active buffer
    fn get_cursor_position(&self) -> u32;

    /// Get cursor line number (1-indexed)
    fn get_cursor_line(&self) -> u32;

    /// Get file path for a buffer
    fn get_buffer_path(&self, buffer_id: u32) -> String;

    /// Get buffer length in bytes
    fn get_buffer_length(&self, buffer_id: u32) -> u32;

    /// Check if buffer is modified
    fn is_buffer_modified(&self, buffer_id: u32) -> bool;

    /// List all buffers as JSON string
    fn list_buffers_json(&self) -> String;

    /// Get primary cursor info as JSON string
    fn get_primary_cursor_json(&self) -> String;

    /// Get all cursors as JSON string
    fn get_all_cursors_json(&self) -> String;

    /// Get viewport info as JSON string
    fn get_viewport_json(&self) -> String;

    /// Get text properties at cursor as JSON string
    fn get_text_properties_at_cursor_json(&self, buffer_id: u32) -> String;

    // ========================================
    // Configuration
    // ========================================

    /// Get merged config as JSON string
    fn get_config(&self) -> String;

    /// Get user config only as JSON string
    fn get_user_config(&self) -> String;

    /// Get config directory path
    fn get_config_dir(&self) -> String;

    /// Get themes directory path
    fn get_themes_dir(&self) -> String;

    // ========================================
    // Theme Operations
    // ========================================

    /// Get theme JSON schema as string
    fn get_theme_schema(&self) -> String;

    /// Get built-in themes as JSON string
    fn get_builtin_themes(&self) -> String;

    /// Delete a user theme (sync, returns success)
    fn delete_theme_sync(&self, name: &str) -> bool;

    // ========================================
    // File System
    // ========================================

    /// Check if file exists
    fn file_exists(&self, path: &str) -> bool;

    /// Read file contents synchronously
    fn read_file_sync(&self, path: &str) -> Option<String>;

    /// Write file contents synchronously
    fn write_file_sync(&self, path: &str, content: &str) -> bool;

    /// Read directory contents as JSON string
    fn read_dir_json(&self, path: &str) -> String;

    // ========================================
    // Environment
    // ========================================

    /// Get environment variable
    fn get_env(&self, name: &str) -> Option<String>;

    /// Get current working directory
    fn get_cwd(&self) -> String;

    // ========================================
    // Path Operations
    // ========================================

    /// Join path segments
    fn path_join(&self, parts: &[String]) -> String;

    /// Get directory name
    fn path_dirname(&self, path: &str) -> String;

    /// Get base name
    fn path_basename(&self, path: &str) -> String;

    /// Get extension
    fn path_extname(&self, path: &str) -> String;

    /// Check if path is absolute
    fn path_is_absolute(&self, path: &str) -> bool;

    // ========================================
    // i18n
    // ========================================

    /// Translate plugin string
    fn plugin_translate(&self, plugin_name: &str, key: &str, args_json: &str) -> String;
}

/// Marker trait for async API methods
///
/// Methods that are async (return Promises in JS) should be implemented
/// separately with the `_start` suffix pattern and callback_id handling.
/// This trait documents which methods have async variants.
pub trait EditorApiAsync {
    /// Get the command sender for async operations
    fn command_sender(&self) -> &Sender<PluginCommand>;

    /// Get the next request ID for callbacks
    fn next_request_id(&self) -> u64;

    // Async methods (implemented as _start functions that return callback_id):
    // - delay(ms) -> delayStart(ms) -> callback_id
    // - spawnProcess(cmd, args, cwd) -> spawnProcessStart(...) -> callback_id
    // - spawnBackgroundProcess(cmd, args, cwd) -> spawnBackgroundProcessStart(...) -> callback_id
    // - getBufferText(buffer_id, start, end) -> getBufferTextStart(...) -> callback_id
    // - readFile(path) -> (uses sync for now)
    // - deleteTheme(name) -> deleteThemeSync (actually sync in QuickJS impl)
    // - createVirtualBuffer(opts) -> createVirtualBufferStart(...) -> callback_id
    // - createVirtualBufferInSplit(opts) -> createVirtualBufferInSplitStart(...) -> callback_id
    // - sendLspRequest(lang, method, params) -> sendLspRequestStart(...) -> callback_id
    // - killProcess(process_id) -> killProcessStart(...) -> callback_id
}

/// List of all API methods for documentation and testing
///
/// This constant documents all methods that should be available in the JS API.
/// It can be used in tests to verify all methods are registered.
pub const API_METHODS: &[(&str, ApiMethodKind)] = &[
    // Status and Logging
    ("setStatus", ApiMethodKind::Sync),
    ("debug", ApiMethodKind::Sync),
    ("error", ApiMethodKind::Sync),
    ("warn", ApiMethodKind::Sync),
    ("info", ApiMethodKind::Sync),

    // Buffer Queries
    ("getActiveBufferId", ApiMethodKind::Sync),
    ("getActiveSplitId", ApiMethodKind::Sync),
    ("getCursorPosition", ApiMethodKind::Sync),
    ("getCursorLine", ApiMethodKind::Sync),
    ("getBufferPath", ApiMethodKind::Sync),
    ("getBufferLength", ApiMethodKind::Sync),
    ("isBufferModified", ApiMethodKind::Sync),
    ("listBuffers", ApiMethodKind::Sync),
    ("getPrimaryCursor", ApiMethodKind::Sync),
    ("getAllCursors", ApiMethodKind::Sync),
    ("getViewport", ApiMethodKind::Sync),
    ("getTextPropertiesAtCursor", ApiMethodKind::Sync),

    // Configuration
    ("getConfig", ApiMethodKind::Sync),
    ("getUserConfig", ApiMethodKind::Sync),
    ("getConfigDir", ApiMethodKind::Sync),
    ("getThemesDir", ApiMethodKind::Sync),
    ("reloadConfig", ApiMethodKind::Sync),

    // Theme
    ("getThemeSchema", ApiMethodKind::Sync),
    ("getBuiltinThemes", ApiMethodKind::Sync),
    ("applyTheme", ApiMethodKind::Sync),
    ("deleteTheme", ApiMethodKind::AsyncSimple),

    // Text Editing
    ("insertText", ApiMethodKind::Sync),
    ("deleteRange", ApiMethodKind::Sync),
    ("insertAtCursor", ApiMethodKind::Sync),
    ("getBufferText", ApiMethodKind::AsyncSimple),

    // Clipboard
    ("setClipboard", ApiMethodKind::Sync),
    ("copyToClipboard", ApiMethodKind::Sync),

    // File Operations
    ("openFile", ApiMethodKind::Sync),
    ("openFileInSplit", ApiMethodKind::Sync),
    ("showBuffer", ApiMethodKind::Sync),
    ("closeBuffer", ApiMethodKind::Sync),
    ("findBufferByPath", ApiMethodKind::Sync),

    // File System
    ("fileExists", ApiMethodKind::Sync),
    ("readFile", ApiMethodKind::Sync), // Sync in QuickJS impl
    ("writeFile", ApiMethodKind::Sync),
    ("readDir", ApiMethodKind::Sync),

    // Environment
    ("getEnv", ApiMethodKind::Sync),
    ("getCwd", ApiMethodKind::Sync),

    // Path Operations
    ("pathJoin", ApiMethodKind::Sync),
    ("pathDirname", ApiMethodKind::Sync),
    ("pathBasename", ApiMethodKind::Sync),
    ("pathExtname", ApiMethodKind::Sync),
    ("pathIsAbsolute", ApiMethodKind::Sync),

    // Commands
    ("registerCommand", ApiMethodKind::Sync), // JS wrapper
    ("_registerCommandInternal", ApiMethodKind::Sync),
    ("unregisterCommand", ApiMethodKind::Sync),
    ("setContext", ApiMethodKind::Sync),
    ("executeAction", ApiMethodKind::Sync),

    // Events
    ("on", ApiMethodKind::Sync),
    ("off", ApiMethodKind::Sync),

    // Prompts
    ("startPrompt", ApiMethodKind::Sync),
    ("startPromptWithInitial", ApiMethodKind::Sync),
    ("setPromptSuggestions", ApiMethodKind::Sync),

    // Overlays
    ("addOverlay", ApiMethodKind::Sync), // JS wrapper
    ("_addOverlayInternal", ApiMethodKind::Sync),
    ("clearNamespace", ApiMethodKind::Sync),
    ("clearAllOverlays", ApiMethodKind::Sync),
    ("setLineIndicator", ApiMethodKind::Sync),
    ("clearLineIndicators", ApiMethodKind::Sync),
    ("refreshLines", ApiMethodKind::Sync),

    // Virtual Buffers
    ("createVirtualBuffer", ApiMethodKind::AsyncSimple),
    ("createVirtualBufferInSplit", ApiMethodKind::AsyncThenable),
    ("setVirtualBufferContent", ApiMethodKind::Sync),

    // Splits
    ("focusSplit", ApiMethodKind::Sync),
    ("setSplitBuffer", ApiMethodKind::Sync),
    ("closeSplit", ApiMethodKind::Sync),
    ("setBufferCursor", ApiMethodKind::Sync),

    // Modes
    ("defineMode", ApiMethodKind::Sync),
    ("setEditorMode", ApiMethodKind::Sync),

    // Process
    ("spawnProcess", ApiMethodKind::AsyncThenable),
    ("spawnBackgroundProcess", ApiMethodKind::AsyncSimple),
    ("killProcess", ApiMethodKind::AsyncSimple),
    ("isProcessRunning", ApiMethodKind::Sync),

    // Async Utilities
    ("delay", ApiMethodKind::AsyncSimple),

    // LSP
    ("sendLspRequest", ApiMethodKind::AsyncSimple),

    // i18n
    ("t", ApiMethodKind::Sync), // JS wrapper
    ("_pluginTranslate", ApiMethodKind::Sync),
];

/// API method kind for documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMethodKind {
    /// Synchronous - returns immediately
    Sync,
    /// Async returning Promise<T>
    AsyncSimple,
    /// Async with cancellation (thenable)
    AsyncThenable,
}
