# QuickJS Plugin Backend

## Overview

Fresh uses QuickJS for its JavaScript plugin runtime, replacing the previous deno_core (V8) backend.

**Benefits:**
- Reduced dependencies (~315 → ~183 crates)
- Faster compilation (no V8 snapshot generation)
- Lighter runtime (~700KB vs multi-MB V8)
- Simple single backend (QuickJS + oxc)

## Status: Complete

| Component | Status |
|-----------|--------|
| QuickJS runtime (rquickjs 0.11) | Complete |
| TypeScript transpilation (oxc 0.108) | Complete |
| ES module bundling | Complete |
| Plugin API (~80+ methods) | Complete |
| Async operations (tokio) | Complete |
| Type-safe bindings | Complete |

**Test coverage:** 52 unit tests + 23 e2e tests passing

## Architecture

### Class-Based API

The plugin API is exposed via `JsEditorApi` using rquickjs class bindings with automatic camelCase conversion.

**Key patterns:**
- `#[rquickjs::class]` - Expose struct to JS
- `#[rquickjs::methods(rename_all = "camelCase")]` - Auto-convert method names
- `rquickjs::function::Opt<T>` - Optional parameters
- `rquickjs::function::Rest<T>` - Variadic arguments
- `rquickjs_serde::to_value()` - Rust → JS conversion

### Async Pattern

Async methods use a callback-based pattern:
1. JS calls `_xxxStart()` → returns callbackId
2. Rust sends `PluginCommand` to app
3. App executes operation, calls `resolve_callback(id, result)`
4. JS Promise resolves

## File Structure

```
src/services/plugins/
├── backend/quickjs_backend.rs  # JsEditorApi implementation
├── api.rs                      # PluginCommand, EditorStateSnapshot
├── transpile.rs                # TypeScript → JS
└── thread.rs                   # Plugin thread runner

crates/fresh-plugin-api-macros/ # TypeScript definition generation
plugins/lib/fresh.d.ts          # Generated TypeScript definitions
```

## Dependencies

- `rquickjs` 0.11 - QuickJS bindings
- `rquickjs-serde` 0.4 - Serde integration
- `oxc_*` 0.108 - TypeScript transpilation
- `fresh-plugin-api-macros` - Proc macros

## Future: Native Async

rquickjs supports native async via `AsyncRuntime`/`AsyncContext` and the `Promised` wrapper. This could replace the `_xxxStart` + JS wrapper pattern but would require architectural changes. The current callback-based pattern works well.

## References

- [rquickjs docs](https://docs.rs/rquickjs/)
- [QuickJS engine](https://bellard.org/quickjs/)
- [oxc project](https://oxc-project.github.io/)
