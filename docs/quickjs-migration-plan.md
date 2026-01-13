# QuickJS Migration Plan

Replace deno_core (V8) + deno_ast with QuickJS + oxc in one shot.

---

## Phase 1: Update Dependencies

### Cargo.toml Changes

**Remove:**
```toml
deno_core = { version = "0.376.0", ... }
deno_ast = { version = "0.51.0", ... }
deno_error = { version = "0.7", ... }
```

**Add:**
```toml
rquickjs = { version = "0.9", features = ["bindgen", "futures", "macro"], optional = true }
oxc_allocator = { version = "0.102", optional = true }
oxc_parser = { version = "0.102", optional = true }
oxc_transformer = { version = "0.102", optional = true }
oxc_codegen = { version = "0.102", optional = true }
oxc_span = { version = "0.102", optional = true }
oxc_semantic = { version = "0.102", optional = true }
```

**Update feature:**
```toml
plugins = ["dep:rquickjs", "dep:oxc_allocator", "dep:oxc_parser", "dep:oxc_transformer", "dep:oxc_codegen", "dep:oxc_span", "dep:oxc_semantic"]
```

---

## Phase 2: Create New Files

### 2.1: `src/services/plugins/transpile.rs`

oxc-based TypeScript transpilation:

```rust
use anyhow::{anyhow, Result};
use oxc_allocator::Allocator;
use oxc_codegen::CodeGenerator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};

pub fn transpile_typescript(source: &str, filename: &str) -> Result<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .map_err(|_| anyhow!("Unknown file type: {}", filename))?;

    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if !parser_ret.errors.is_empty() {
        return Err(anyhow!("Parse errors: {:?}", parser_ret.errors));
    }

    let mut program = parser_ret.program;
    let semantic_ret = SemanticBuilder::new().build(&program);
    let (symbols, scopes) = semantic_ret.semantic.into_symbol_table_and_scope_tree();

    let _ = Transformer::new(&allocator, std::path::Path::new(filename), &TransformOptions::default())
        .build_with_symbols_and_scopes(symbols, scopes, &mut program);

    Ok(CodeGenerator::new().build(&program).code)
}
```

### 2.2: `src/services/plugins/backend/mod.rs`

```rust
mod quickjs_backend;
pub use quickjs_backend::QuickJsBackend;
```

### 2.3: `src/services/plugins/backend/quickjs_backend.rs`

Core structure (~800-1000 lines):

```rust
use crate::services::plugins::api::{EditorStateSnapshot, PluginCommand};
use crate::services::plugins::transpile::transpile_typescript;
use anyhow::Result;
use rquickjs::{Context, Function, Object, Runtime};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};

pub struct QuickJsBackend {
    runtime: Runtime,
    context: Context,
    event_handlers: HashMap<String, Vec<rquickjs::Persistent<Function<'static>>>>,
    state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
    command_sender: mpsc::Sender<PluginCommand>,
}

impl QuickJsBackend {
    pub fn new(
        state_snapshot: Arc<RwLock<EditorStateSnapshot>>,
        command_sender: mpsc::Sender<PluginCommand>,
    ) -> Result<Self> { ... }

    pub fn load_plugin(&mut self, path: &str, name: &str) -> Result<()> { ... }
    pub fn emit(&mut self, event: &str, data: &str) -> Result<()> { ... }
    pub fn has_handlers(&self, event: &str) -> bool { ... }
    pub fn execute_action(&mut self, name: &str) -> Result<()> { ... }
}
```

**Editor API to implement (port from runtime.rs):**

| Method | Implementation |
|--------|----------------|
| `setStatus(msg)` | Send `PluginCommand::SetStatus` |
| `debug(msg)` | `tracing::debug!` |
| `getActiveBufferId()` | Read from `state_snapshot` |
| `getBufferPath(id)` | Read from `state_snapshot` |
| `getCursorPosition(id)` | Read from `state_snapshot` |
| `insertText(id, pos, text)` | Send `PluginCommand::InsertText` |
| `deleteRange(id, start, end)` | Send `PluginCommand::DeleteRange` |
| `registerCommand(name, fn)` | Store callback, send registration |
| `on(event, fn)` | Add to `event_handlers` map |
| `off(event, fn)` | Remove from `event_handlers` map |
| `openFile(path)` | Send `PluginCommand::OpenFile` |
| `fileExists(path)` | `std::path::Path::new(path).exists()` |
| `readFile(path)` | `std::fs::read_to_string(path)` |
| `writeFile(path, content)` | `std::fs::write(path, content)` |
| `pathJoin(...parts)` | `std::path::Path` operations |
| `getEnv(key)` | `std::env::var(key)` |
| `getCwd()` | `std::env::current_dir()` |
| `copyToClipboard(text)` | Send `PluginCommand::CopyToClipboard` |

**Stub methods (log warning, return default):**
- `spawnProcess`, `addOverlay`, `clearNamespace`, `defineMode`
- `startPrompt`, `setPromptSuggestions`, `refreshLines`
- `createVirtualBufferInSplit`, `setVirtualBufferContent`
- `closeSplit`, `setSplitBuffer`, `setLineIndicator`, `clearLineIndicators`

---

## Phase 3: Update Existing Files

### 3.1: `src/services/plugins/mod.rs`

Add:
```rust
#[cfg(feature = "plugins")]
mod transpile;
#[cfg(feature = "plugins")]
mod backend;
```

### 3.2: `src/services/plugins/thread.rs`

Replace:
```rust
use crate::services::plugins::runtime::{TsPluginInfo, TypeScriptRuntime};
```

With:
```rust
use crate::services::plugins::backend::QuickJsBackend;

pub struct TsPluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
}
```

Update `PluginThreadHandle::spawn()`:
```rust
// Old: TypeScriptRuntime::with_state_and_responses(...)
// New: QuickJsBackend::new(state_snapshot, command_sender)
```

Update function signatures:
```rust
// Old: Rc<RefCell<TypeScriptRuntime>>
// New: Rc<RefCell<QuickJsBackend>>
```

### 3.3: `tests/common/harness.rs`

Remove any V8 initialization code.

---

## Phase 4: Delete Files

- `src/services/plugins/runtime.rs` (265KB) - entire file

---

## Phase 5: ES Module Support (for clangd_support.ts)

Add to `transpile.rs`:

```rust
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn bundle_module(entry: &Path) -> Result<String> {
    let mut visited = HashSet::new();
    let mut output = String::new();
    bundle_recursive(entry, &mut visited, &mut output)?;
    Ok(output)
}

fn bundle_recursive(path: &Path, visited: &mut HashSet<PathBuf>, out: &mut String) -> Result<()> {
    if !visited.insert(path.to_path_buf()) { return Ok(()); }

    let source = std::fs::read_to_string(path)?;
    for import in extract_imports(&source) {
        let resolved = path.parent().unwrap().join(&import);
        let resolved = if resolved.exists() { resolved }
            else if resolved.with_extension("ts").exists() { resolved.with_extension("ts") }
            else { resolved.with_extension("js") };
        bundle_recursive(&resolved, visited, out)?;
    }

    let stripped = strip_imports(&source);
    out.push_str(&transpile_typescript(&stripped, path.to_str().unwrap())?);
    out.push('\n');
    Ok(())
}

fn extract_imports(source: &str) -> Vec<String> {
    regex::Regex::new(r#"import\s+.*?\s+from\s+['"](\./[^'"]+)['"]"#)
        .unwrap()
        .captures_iter(source)
        .map(|c| c[1].to_string())
        .collect()
}

fn strip_imports(source: &str) -> String {
    regex::Regex::new(r#"import\s+.*?\s+from\s+['"][^'"]+['"];\s*"#)
        .unwrap()
        .replace_all(source, "")
        .to_string()
}
```

Update `QuickJsBackend::load_plugin()` to detect imports and use bundler.

---

## Summary

| Action | File |
|--------|------|
| **Create** | `src/services/plugins/transpile.rs` |
| **Create** | `src/services/plugins/backend/mod.rs` |
| **Create** | `src/services/plugins/backend/quickjs_backend.rs` |
| **Modify** | `Cargo.toml` |
| **Modify** | `src/services/plugins/mod.rs` |
| **Modify** | `src/services/plugins/thread.rs` |
| **Modify** | `tests/common/harness.rs` |
| **Delete** | `src/services/plugins/runtime.rs` |

---

## Validation

```bash
cargo build --features plugins
cargo test --features plugins
# Load editor and verify plugins work
```
