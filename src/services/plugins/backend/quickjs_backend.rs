//! QuickJS JavaScript runtime backend for TypeScript plugins
//!
//! This module provides a JavaScript runtime using QuickJS for executing
//! TypeScript plugins. TypeScript is transpiled to JavaScript using oxc.

use crate::config_io::DirectoryContext;
use crate::input::commands::{Command, CommandSource};
use crate::input::keybindings::Action;
use crate::model::event::{BufferId, SplitId};
use crate::services::plugins::api::{EditorStateSnapshot, PluginCommand, PluginResponse};
use crate::services::plugins::transpile::{bundle_module, has_es_imports, transpile_typescript};
use crate::view::overlay::OverlayNamespace;
use anyhow::{anyhow, Result};
use rquickjs::{Context, Function, Object, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{mpsc, Arc, RwLock};

/// Pending response senders type alias
pub type PendingResponses =
    Arc<std::sync::Mutex<HashMap<u64, tokio::sync::oneshot::Sender<PluginResponse>>>>;

/// Information about a loaded plugin
#[derive(Debug, Clone)]
pub struct TsPluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
}

/// QuickJS-based JavaScript runtime for plugins
pub struct QuickJsBackend {
    runtime: Runtime,
    context: Context,
    /// Event handlers: event_name -> list of handler function names
    event_handlers: Rc<RefCell<HashMap<String, Vec<String>>>>,
    /// Registered actions: action_name -> handler function name
    registered_actions: Rc<RefCell<HashMap<String, String>>>,
    /// Editor state snapshot (read-only access)
    state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
    /// Command sender for write operations
    command_sender: mpsc::Sender<PluginCommand>,
    /// Pending response senders for async operations
    pending_responses: PendingResponses,
    /// Next request ID for async operations
    next_request_id: Rc<RefCell<u64>>,
    /// Directory context for system paths
    dir_context: DirectoryContext,
}

impl QuickJsBackend {
    /// Create a new QuickJS backend (standalone, for testing)
    pub fn new() -> Result<Self> {
        let (tx, _rx) = mpsc::channel();
        let state_snapshot = Arc::new(RwLock::new(EditorStateSnapshot::new()));
        let dir_context = DirectoryContext::for_testing(Path::new("/tmp"));
        Self::with_state(state_snapshot, tx, dir_context)
    }

    /// Create a new QuickJS backend with editor state
    pub fn with_state(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: mpsc::Sender<PluginCommand>,
        dir_context: DirectoryContext,
    ) -> Result<Self> {
        let pending_responses: PendingResponses = Arc::new(std::sync::Mutex::new(HashMap::new()));
        Self::with_state_and_responses(state_snapshot, command_sender, pending_responses, dir_context)
    }

    /// Create a new QuickJS backend with editor state and shared pending responses
    pub fn with_state_and_responses(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: mpsc::Sender<PluginCommand>,
        pending_responses: PendingResponses,
        dir_context: DirectoryContext,
    ) -> Result<Self> {
        tracing::debug!("QuickJsBackend::new: creating QuickJS runtime");

        let runtime = Runtime::new().map_err(|e| anyhow!("Failed to create QuickJS runtime: {}", e))?;
        let context = Context::full(&runtime).map_err(|e| anyhow!("Failed to create QuickJS context: {}", e))?;

        let event_handlers = Rc::new(RefCell::new(HashMap::new()));
        let registered_actions = Rc::new(RefCell::new(HashMap::new()));
        let next_request_id = Rc::new(RefCell::new(1u64));

        let mut backend = Self {
            runtime,
            context,
            event_handlers,
            registered_actions,
            state_snapshot,
            command_sender,
            pending_responses,
            next_request_id,
            dir_context,
        };

        backend.setup_global_api()?;

        tracing::debug!("QuickJsBackend::new: runtime created successfully");
        Ok(backend)
    }

    /// Set up the global editor API in the JavaScript context
    fn setup_global_api(&mut self) -> Result<()> {
        let state_snapshot = Arc::clone(&self.state_snapshot);
        let command_sender = self.command_sender.clone();
        let event_handlers = Rc::clone(&self.event_handlers);
        let registered_actions = Rc::clone(&self.registered_actions);
        let next_request_id = Rc::clone(&self.next_request_id);
        let dir_context = self.dir_context.clone();

        self.context.with(|ctx| {
            let globals = ctx.globals();

            // Create the editor object
            let editor = Object::new(ctx.clone())?;

            // === Logging ===
            editor.set("debug", Function::new(ctx.clone(), |msg: String| {
                tracing::debug!("Plugin: {}", msg);
            })?)?;

            editor.set("info", Function::new(ctx.clone(), |msg: String| {
                tracing::info!("Plugin: {}", msg);
            })?)?;

            editor.set("warn", Function::new(ctx.clone(), |msg: String| {
                tracing::warn!("Plugin: {}", msg);
            })?)?;

            editor.set("error", Function::new(ctx.clone(), |msg: String| {
                tracing::error!("Plugin: {}", msg);
            })?)?;

            // === Status ===
            let cmd_sender = command_sender.clone();
            editor.set("setStatus", Function::new(ctx.clone(), move |msg: String| {
                let _ = cmd_sender.send(PluginCommand::SetStatus { message: msg });
            })?)?;

            // === Clipboard ===
            let cmd_sender = command_sender.clone();
            editor.set("copyToClipboard", Function::new(ctx.clone(), move |text: String| {
                let _ = cmd_sender.send(PluginCommand::SetClipboard { text });
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("setClipboard", Function::new(ctx.clone(), move |text: String| {
                let _ = cmd_sender.send(PluginCommand::SetClipboard { text });
            })?)?;

            // === Buffer queries ===
            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getActiveBufferId", Function::new(ctx.clone(), move || -> u32 {
                snapshot.read().map(|s| s.active_buffer_id.0 as u32).unwrap_or(0)
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getActiveSplitId", Function::new(ctx.clone(), move || -> u32 {
                snapshot.read().map(|s| s.active_split_id as u32).unwrap_or(0)
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getCursorPosition", Function::new(ctx.clone(), move || -> u32 {
                snapshot.read()
                    .ok()
                    .and_then(|s| s.primary_cursor.as_ref().map(|c| c.position as u32))
                    .unwrap_or(0)
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getBufferPath", Function::new(ctx.clone(), move |buffer_id: u32| -> String {
                if let Ok(s) = snapshot.read() {
                    if let Some(b) = s.buffers.get(&BufferId(buffer_id as usize)) {
                        if let Some(p) = &b.path {
                            return p.to_string_lossy().to_string();
                        }
                    }
                }
                String::new()
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getBufferLength", Function::new(ctx.clone(), move |buffer_id: u32| -> u32 {
                if let Ok(s) = snapshot.read() {
                    if let Some(b) = s.buffers.get(&BufferId(buffer_id as usize)) {
                        return b.length as u32;
                    }
                }
                0
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("isBufferModified", Function::new(ctx.clone(), move |buffer_id: u32| -> bool {
                if let Ok(s) = snapshot.read() {
                    if let Some(b) = s.buffers.get(&BufferId(buffer_id as usize)) {
                        return b.modified;
                    }
                }
                false
            })?)?;

            // === Text editing ===
            let cmd_sender = command_sender.clone();
            editor.set("insertText", Function::new(ctx.clone(), move |buffer_id: u32, position: u32, text: String| -> bool {
                cmd_sender.send(PluginCommand::InsertText {
                    buffer_id: BufferId(buffer_id as usize),
                    position: position as usize,
                    text,
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("deleteRange", Function::new(ctx.clone(), move |buffer_id: u32, start: u32, end: u32| -> bool {
                cmd_sender.send(PluginCommand::DeleteRange {
                    buffer_id: BufferId(buffer_id as usize),
                    range: (start as usize)..(end as usize),
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("insertAtCursor", Function::new(ctx.clone(), move |text: String| -> bool {
                cmd_sender.send(PluginCommand::InsertAtCursor { text }).is_ok()
            })?)?;

            // === File operations ===
            let cmd_sender = command_sender.clone();
            editor.set("openFile", Function::new(ctx.clone(), move |path: String, line: Option<u32>, column: Option<u32>| -> bool {
                cmd_sender.send(PluginCommand::OpenFileAtLocation {
                    path: PathBuf::from(path),
                    line: line.map(|l| l as usize),
                    column: column.map(|c| c as usize),
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("showBuffer", Function::new(ctx.clone(), move |buffer_id: u32| -> bool {
                cmd_sender.send(PluginCommand::ShowBuffer {
                    buffer_id: BufferId(buffer_id as usize),
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("closeBuffer", Function::new(ctx.clone(), move |buffer_id: u32| -> bool {
                cmd_sender.send(PluginCommand::CloseBuffer {
                    buffer_id: BufferId(buffer_id as usize),
                }).is_ok()
            })?)?;

            // === Event handling ===
            let handlers = Rc::clone(&event_handlers);
            editor.set("on", Function::new(ctx.clone(), move |event_name: String, handler_name: String| {
                let mut h = handlers.borrow_mut();
                h.entry(event_name).or_default().push(handler_name);
            })?)?;

            let handlers = Rc::clone(&event_handlers);
            editor.set("off", Function::new(ctx.clone(), move |event_name: String, handler_name: String| {
                let mut h = handlers.borrow_mut();
                if let Some(handlers) = h.get_mut(&event_name) {
                    handlers.retain(|h| h != &handler_name);
                }
            })?)?;

            // === Command registration ===
            let cmd_sender = command_sender.clone();
            let actions = Rc::clone(&registered_actions);
            editor.set("registerCommand", Function::new(ctx.clone(), move |name: String, handler_name: String, description: Option<String>| -> bool {
                // Store action handler
                actions.borrow_mut().insert(name.clone(), handler_name.clone());

                // Register with editor
                let command = Command {
                    name: name.clone(),
                    description: description.unwrap_or_else(|| name.clone()),
                    action: Action::PluginAction(name),
                    contexts: vec![],
                    custom_contexts: vec![],
                    source: CommandSource::Plugin(handler_name),
                };

                cmd_sender.send(PluginCommand::RegisterCommand { command }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("unregisterCommand", Function::new(ctx.clone(), move |name: String| -> bool {
                cmd_sender.send(PluginCommand::UnregisterCommand { name }).is_ok()
            })?)?;

            // === Context ===
            let cmd_sender = command_sender.clone();
            editor.set("setContext", Function::new(ctx.clone(), move |name: String, active: bool| -> bool {
                cmd_sender.send(PluginCommand::SetContext { name, active }).is_ok()
            })?)?;

            // === Environment ===
            editor.set("getEnv", Function::new(ctx.clone(), |name: String| -> Option<String> {
                std::env::var(&name).ok()
            })?)?;

            // getCwd returns current working directory (from std::env, not dir_context)
            editor.set("getCwd", Function::new(ctx.clone(), || -> String {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            })?)?;

            // === Path operations ===
            editor.set("pathJoin", Function::new(ctx.clone(), |parts: Vec<String>| -> String {
                let mut path = PathBuf::new();
                for part in parts {
                    if Path::new(&part).is_absolute() {
                        path = PathBuf::from(part);
                    } else {
                        path.push(part);
                    }
                }
                path.to_string_lossy().to_string()
            })?)?;

            editor.set("pathDirname", Function::new(ctx.clone(), |path: String| -> String {
                Path::new(&path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default()
            })?)?;

            editor.set("pathBasename", Function::new(ctx.clone(), |path: String| -> String {
                Path::new(&path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            })?)?;

            editor.set("pathExtname", Function::new(ctx.clone(), |path: String| -> String {
                Path::new(&path)
                    .extension()
                    .map(|s| format!(".{}", s.to_string_lossy()))
                    .unwrap_or_default()
            })?)?;

            editor.set("pathIsAbsolute", Function::new(ctx.clone(), |path: String| -> bool {
                Path::new(&path).is_absolute()
            })?)?;

            // === File system ===
            editor.set("fileExists", Function::new(ctx.clone(), |path: String| -> bool {
                Path::new(&path).exists()
            })?)?;

            editor.set("readFile", Function::new(ctx.clone(), |path: String| -> Option<String> {
                std::fs::read_to_string(&path).ok()
            })?)?;

            editor.set("writeFile", Function::new(ctx.clone(), |path: String, content: String| -> bool {
                std::fs::write(&path, content).is_ok()
            })?)?;

            // === Config ===
            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getConfig", Function::new(ctx.clone(), move || -> String {
                snapshot.read()
                    .map(|s| s.config.to_string())
                    .unwrap_or_else(|_| "{}".to_string())
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getUserConfig", Function::new(ctx.clone(), move || -> String {
                snapshot.read()
                    .map(|s| s.user_config.to_string())
                    .unwrap_or_else(|_| "{}".to_string())
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("reloadConfig", Function::new(ctx.clone(), move || {
                let _ = cmd_sender.send(PluginCommand::ReloadConfig);
            })?)?;

            let dir_ctx = dir_context.clone();
            editor.set("getConfigDir", Function::new(ctx.clone(), move || -> String {
                dir_ctx.config_dir.to_string_lossy().to_string()
            })?)?;

            let dir_ctx = dir_context.clone();
            editor.set("getThemesDir", Function::new(ctx.clone(), move || -> String {
                dir_ctx.config_dir.join("themes").to_string_lossy().to_string()
            })?)?;

            // === Theme ===
            let cmd_sender = command_sender.clone();
            editor.set("applyTheme", Function::new(ctx.clone(), move |theme_name: String| -> bool {
                cmd_sender.send(PluginCommand::ApplyTheme { theme_name }).is_ok()
            })?)?;

            // === Overlays (stub with warning) ===
            // Note: addOverlay takes a JSON config string for simplicity with QuickJS
            editor.set("addOverlay", Function::new(ctx.clone(), |_config_json: String| -> String {
                tracing::warn!("addOverlay: stub implementation");
                "stub-handle".to_string()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("clearNamespace", Function::new(ctx.clone(), move |buffer_id: u32, namespace: String| -> bool {
                cmd_sender.send(PluginCommand::ClearNamespace {
                    buffer_id: BufferId(buffer_id as usize),
                    namespace: OverlayNamespace::from_string(namespace),
                }).is_ok()
            })?)?;

            editor.set("clearAllOverlays", Function::new(ctx.clone(), |_buffer_id: u32| -> bool {
                tracing::warn!("clearAllOverlays: stub implementation");
                true
            })?)?;

            // === Prompt (stub with warning) ===
            let cmd_sender = command_sender.clone();
            editor.set("startPrompt", Function::new(ctx.clone(), move |label: String, prompt_type: String| -> bool {
                cmd_sender.send(PluginCommand::StartPrompt { label, prompt_type }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("startPromptWithInitial", Function::new(ctx.clone(), move |label: String, prompt_type: String, initial_value: String| -> bool {
                cmd_sender.send(PluginCommand::StartPromptWithInitial { label, prompt_type, initial_value }).is_ok()
            })?)?;

            editor.set("setPromptSuggestions", Function::new(ctx.clone(), |_suggestions: Vec<String>| -> bool {
                tracing::warn!("setPromptSuggestions: stub implementation (needs full Suggestion struct)");
                true
            })?)?;

            // === Mode definition (stub with warning) ===
            // Note: defineMode takes a JSON config string for simplicity with QuickJS
            editor.set("defineMode", Function::new(ctx.clone(), |_config_json: String| -> bool {
                tracing::warn!("defineMode: stub implementation");
                true
            })?)?;

            // === Virtual buffers (stub with warning) ===
            editor.set("createVirtualBufferInSplit", Function::new(ctx.clone(), |_options: String| -> u32 {
                tracing::warn!("createVirtualBufferInSplit: stub implementation");
                0
            })?)?;

            editor.set("setVirtualBufferContent", Function::new(ctx.clone(), |_buffer_id: u32, _entries: Vec<String>| -> bool {
                tracing::warn!("setVirtualBufferContent: stub implementation");
                true
            })?)?;

            editor.set("getTextPropertiesAtCursor", Function::new(ctx.clone(), |_buffer_id: u32| -> Option<String> {
                tracing::warn!("getTextPropertiesAtCursor: stub implementation");
                None
            })?)?;

            // === Split operations ===
            let cmd_sender = command_sender.clone();
            editor.set("closeSplit", Function::new(ctx.clone(), move |split_id: u32| -> bool {
                cmd_sender.send(PluginCommand::CloseSplit {
                    split_id: SplitId(split_id as usize),
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("setSplitBuffer", Function::new(ctx.clone(), move |split_id: u32, buffer_id: u32| -> bool {
                cmd_sender.send(PluginCommand::SetSplitBuffer {
                    split_id: SplitId(split_id as usize),
                    buffer_id: BufferId(buffer_id as usize),
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("focusSplit", Function::new(ctx.clone(), move |split_id: u32| -> bool {
                cmd_sender.send(PluginCommand::FocusSplit {
                    split_id: SplitId(split_id as usize),
                }).is_ok()
            })?)?;

            let cmd_sender = command_sender.clone();
            editor.set("setBufferCursor", Function::new(ctx.clone(), move |buffer_id: u32, position: u32| -> bool {
                cmd_sender.send(PluginCommand::SetBufferCursor {
                    buffer_id: BufferId(buffer_id as usize),
                    position: position as usize,
                }).is_ok()
            })?)?;

            // === Line indicators (stub) ===
            // Note: setLineIndicator takes a JSON config string for simplicity with QuickJS
            editor.set("setLineIndicator", Function::new(ctx.clone(), |_config_json: String| -> bool {
                tracing::warn!("setLineIndicator: stub implementation");
                true
            })?)?;

            editor.set("clearLineIndicators", Function::new(ctx.clone(), |_buffer_id: u32, _namespace: String| -> bool {
                tracing::warn!("clearLineIndicators: stub implementation");
                true
            })?)?;

            // === Process spawning (stub) ===
            editor.set("spawnProcess", Function::new(ctx.clone(), |_command: String, _args: Vec<String>, _cwd: Option<String>| -> String {
                tracing::warn!("spawnProcess: stub implementation - returns empty result");
                r#"{"stdout":"","stderr":"","exitCode":1}"#.to_string()
            })?)?;

            // === Refresh ===
            let cmd_sender = command_sender.clone();
            editor.set("refreshLines", Function::new(ctx.clone(), move |buffer_id: u32| -> bool {
                cmd_sender.send(PluginCommand::RefreshLines {
                    buffer_id: BufferId(buffer_id as usize),
                }).is_ok()
            })?)?;

            // === i18n ===
            editor.set("getCurrentLocale", Function::new(ctx.clone(), || -> String {
                crate::i18n::current_locale()
            })?)?;

            // === Editor mode ===
            let cmd_sender = command_sender.clone();
            editor.set("setEditorMode", Function::new(ctx.clone(), move |mode: Option<String>| -> bool {
                cmd_sender.send(PluginCommand::SetEditorMode { mode }).is_ok()
            })?)?;

            let snapshot = Arc::clone(&state_snapshot);
            editor.set("getEditorMode", Function::new(ctx.clone(), move || -> Option<String> {
                snapshot.read().ok().and_then(|s| s.editor_mode.clone())
            })?)?;

            // Set editor as global
            globals.set("editor", editor)?;

            // Set up getEditor function for plugin initialization
            globals.set("getEditor", Function::new(ctx.clone(), || -> () {
                // In QuickJS, editor is already global, so getEditor() is a no-op
                // Plugins can use `editor` directly
            })?)?;

            // Provide console.log for debugging
            let console = Object::new(ctx.clone())?;
            console.set("log", Function::new(ctx.clone(), |args: Vec<String>| {
                tracing::info!("console.log: {}", args.join(" "));
            })?)?;
            console.set("warn", Function::new(ctx.clone(), |args: Vec<String>| {
                tracing::warn!("console.warn: {}", args.join(" "));
            })?)?;
            console.set("error", Function::new(ctx.clone(), |args: Vec<String>| {
                tracing::error!("console.error: {}", args.join(" "));
            })?)?;
            globals.set("console", console)?;

            Ok::<_, rquickjs::Error>(())
        }).map_err(|e| anyhow!("Failed to set up global API: {}", e))?;

        Ok(())
    }

    /// Load and execute a TypeScript/JavaScript plugin from a file path
    pub async fn load_module_with_source(&mut self, path: &str, _plugin_source: &str) -> Result<()> {
        let path_buf = PathBuf::from(path);
        let source = std::fs::read_to_string(&path_buf)
            .map_err(|e| anyhow!("Failed to read plugin {}: {}", path, e))?;

        // Check for ES imports
        if has_es_imports(&source) {
            // Try to bundle
            match bundle_module(&path_buf) {
                Ok(bundled) => {
                    self.execute_js(&bundled, path)?;
                }
                Err(e) => {
                    tracing::warn!(
                        "Plugin {} uses ES imports but bundling failed: {}. Skipping.",
                        path, e
                    );
                    return Ok(()); // Skip plugins with unresolvable imports
                }
            }
        } else {
            // Transpile and execute
            let filename = path_buf.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("plugin.ts");

            let js_code = if filename.ends_with(".ts") {
                transpile_typescript(&source, filename)?
            } else {
                source
            };

            self.execute_js(&js_code, path)?;
        }

        Ok(())
    }

    /// Execute JavaScript code in the context
    fn execute_js(&mut self, code: &str, source_name: &str) -> Result<()> {
        // Wrap in IIFE for scope isolation
        let wrapped = format!(
            "(function() {{\n{}\n}})();",
            code
        );

        self.context.with(|ctx| {
            ctx.eval::<(), _>(wrapped.as_bytes())
                .map_err(|e| anyhow!("JS error in {}: {}", source_name, e))
        })
    }

    /// Emit an event to all registered handlers
    pub async fn emit(&mut self, event_name: &str, event_data: &str) -> Result<bool> {
        let handlers = self.event_handlers.borrow().get(event_name).cloned();

        if let Some(handler_names) = handlers {
            if handler_names.is_empty() {
                return Ok(true);
            }

            for handler_name in &handler_names {
                let code = format!(
                    r#"
                    (function() {{
                        try {{
                            const data = JSON.parse({});
                            if (typeof globalThis.{} === 'function') {{
                                globalThis.{}(data);
                            }}
                        }} catch (e) {{
                            console.error('Handler {} error:', e);
                        }}
                    }})();
                    "#,
                    serde_json::to_string(event_data)?,
                    handler_name,
                    handler_name,
                    handler_name
                );

                self.context.with(|ctx| {
                    if let Err(e) = ctx.eval::<(), _>(code.as_bytes()) {
                        tracing::error!("Error calling handler {}: {}", handler_name, e);
                    }
                });
            }
        }

        Ok(true)
    }

    /// Check if any handlers are registered for an event
    pub fn has_handlers(&self, event_name: &str) -> bool {
        self.event_handlers
            .borrow()
            .get(event_name)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Execute a registered action by name
    pub async fn execute_action(&mut self, action_name: &str) -> Result<()> {
        let handler_name = self.registered_actions.borrow().get(action_name).cloned();

        if let Some(handler) = handler_name {
            let code = format!(
                r#"
                (function() {{
                    try {{
                        if (typeof globalThis.{} === 'function') {{
                            globalThis.{}();
                        }}
                    }} catch (e) {{
                        console.error('Action {} error:', e);
                    }}
                }})();
                "#,
                handler, handler, action_name
            );

            self.context.with(|ctx| {
                if let Err(e) = ctx.eval::<(), _>(code.as_bytes()) {
                    tracing::error!("Error executing action {}: {}", action_name, e);
                }
            });
        } else {
            tracing::warn!("No handler found for action: {}", action_name);
        }

        Ok(())
    }

    /// Poll the event loop once (QuickJS is synchronous, so this is a no-op)
    pub fn poll_event_loop_once(&mut self) -> bool {
        // QuickJS doesn't have an async event loop like V8
        // Return false to indicate no pending work
        false
    }

    /// Send a status message to the editor
    pub fn send_status(&self, message: String) {
        let _ = self.command_sender.send(PluginCommand::SetStatus { message });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quickjs_backend_creation() {
        let backend = QuickJsBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_execute_simple_js() {
        let mut backend = QuickJsBackend::new().unwrap();
        let result = backend.execute_js("const x = 1 + 2;", "test.js");
        assert!(result.is_ok());
    }

    #[test]
    fn test_event_handler_registration() {
        let backend = QuickJsBackend::new().unwrap();

        // Initially no handlers
        assert!(!backend.has_handlers("test_event"));

        // Register a handler
        backend.event_handlers.borrow_mut()
            .entry("test_event".to_string())
            .or_default()
            .push("testHandler".to_string());

        // Now has handlers
        assert!(backend.has_handlers("test_event"));
    }
}
