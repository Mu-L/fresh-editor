//! Proc macros for type-safe plugin API bindings
//!
//! This crate provides the `#[plugin_api_impl]` attribute macro that generates
//! TypeScript definitions from a QuickJS impl block.
//!
//! # Usage
//!
//! ```rust,ignore
//! use fresh_plugin_api_macros::plugin_api_impl;
//!
//! #[plugin_api_impl]
//! #[rquickjs::methods(rename_all = "camelCase")]
//! impl JsEditorApi {
//!     /// Get the active buffer ID
//!     pub fn get_active_buffer_id(&self) -> u32 { ... }
//!
//!     /// Create a virtual buffer (async)
//!     #[qjs(rename = "_createVirtualBufferStart")]
//!     pub fn create_virtual_buffer_start(&self, ...) -> u64 { ... }
//! }
//! ```
//!
//! # Async Method Detection
//!
//! Methods are detected as async by:
//! 1. Having `#[qjs(rename = "_...Start")]` attribute, OR
//! 2. Having `#[plugin_api(async_promise)]` or `#[plugin_api(async_thenable)]` attribute
//!
//! For async methods:
//! - `_xxxStart` becomes `xxx()` in TypeScript
//! - Return type `u64` (callback_id) becomes `Promise<T>` or `ProcessHandle<T>`
//!
//! # Type Mapping
//!
//! - `rquickjs::Ctx<'js>` → skipped (not in TS signature)
//! - `rquickjs::function::Opt<T>` → optional parameter
//! - `rquickjs::function::Rest<T>` → variadic parameter
//! - `rquickjs::Result<T>` → `T` (unwrapped)
//! - `rquickjs::Object<'js>` → use `#[plugin_api(ts_type = "...")]`

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, FnArg, GenericArgument, ImplItem, ImplItemFn, ItemImpl, Meta,
    Pat, PathArguments, ReturnType, Type,
};

// ============================================================================
// API Method Kind
// ============================================================================

/// API method kind determined by attributes and naming
#[derive(Debug, Clone, PartialEq)]
enum ApiKind {
    /// Synchronous method - returns value directly
    Sync,
    /// Async method that returns a simple Promise<T>
    AsyncPromise,
    /// Async method that returns a Thenable<T> (with .kill() support)
    AsyncThenable,
}

// ============================================================================
// Parsed API Method
// ============================================================================

/// Parsed API method information
#[derive(Debug)]
struct ApiMethod {
    /// JavaScript method name (camelCase, without _Start suffix for async)
    js_name: String,
    /// Method kind (sync/async)
    kind: ApiKind,
    /// Parameters: (name, type_string, is_optional, is_variadic)
    params: Vec<ParamInfo>,
    /// Return type as TypeScript string
    ts_return_type: String,
    /// Doc comment for TypeScript
    doc_comment: String,
}

/// Parsed parameter info
#[derive(Debug)]
struct ParamInfo {
    /// Parameter name in camelCase
    name: String,
    /// TypeScript type
    ts_type: String,
    /// Whether this is optional (from Opt<T>)
    is_optional: bool,
    /// Whether this is variadic (from Rest<T>)
    is_variadic: bool,
}

// ============================================================================
// String Utilities
// ============================================================================

/// Convert snake_case to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

// ============================================================================
// Attribute Parsing
// ============================================================================

/// Extract doc comments from attributes
fn extract_doc_comment(attrs: &[Attribute]) -> String {
    let mut docs = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        docs.push(lit_str.value().trim().to_string());
                    }
                }
            }
        }
    }
    docs.join("\n")
}

/// Check for #[plugin_api(skip)] attribute
fn should_skip(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("plugin_api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("skip") {
                    return true;
                }
            }
        }
    }
    false
}

/// Get custom JS name from #[qjs(rename = "...")] or #[plugin_api(js_name = "...")]
fn get_custom_js_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        // Check #[qjs(rename = "...")]
        if attr.path().is_ident("qjs") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if let Some(start) = tokens.find("rename") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let after_eq = rest[eq_pos + 1..].trim();
                        if after_eq.starts_with('"') {
                            if let Some(end_quote) = after_eq[1..].find('"') {
                                return Some(after_eq[1..end_quote + 1].to_string());
                            }
                        }
                    }
                }
            }
        }
        // Check #[plugin_api(js_name = "...")]
        if attr.path().is_ident("plugin_api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if let Some(start) = tokens.find("js_name") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let after_eq = rest[eq_pos + 1..].trim();
                        if after_eq.starts_with('"') {
                            if let Some(end_quote) = after_eq[1..].find('"') {
                                return Some(after_eq[1..end_quote + 1].to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check for async_thenable attribute
fn is_async_thenable(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("plugin_api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("async_thenable") {
                    return true;
                }
            }
        }
    }
    false
}

/// Check for async_promise attribute
fn is_async_promise(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("plugin_api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("async_promise") {
                    return true;
                }
            }
        }
    }
    false
}

/// Get custom TypeScript type from #[plugin_api(ts_type = "...")]
fn get_custom_ts_type(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("plugin_api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if let Some(start) = tokens.find("ts_type") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let after_eq = rest[eq_pos + 1..].trim();
                        if after_eq.starts_with('"') {
                            if let Some(end_quote) = after_eq[1..].find('"') {
                                return Some(after_eq[1..end_quote + 1].to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get custom TypeScript return type from #[plugin_api(ts_return = "...")]
fn get_custom_ts_return(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("plugin_api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                if let Some(start) = tokens.find("ts_return") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let after_eq = rest[eq_pos + 1..].trim();
                        if after_eq.starts_with('"') {
                            if let Some(end_quote) = after_eq[1..].find('"') {
                                return Some(after_eq[1..end_quote + 1].to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// Type Utilities
// ============================================================================

/// Extract the inner type from a generic wrapper like Option<T>, Vec<T>, etc.
fn extract_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        }
    }
    None
}

/// Get the last path segment name (e.g., "Opt" from "rquickjs::function::Opt")
fn get_type_name(ty: &Type) -> Option<String> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return Some(segment.ident.to_string());
        }
    }
    None
}

/// Check if type is a QuickJS context type (should be skipped)
fn is_quickjs_ctx(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        // Check for Ctx<'js> pattern
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Ctx" {
                return true;
            }
        }
        // Check for rquickjs::Ctx path
        let path_str = type_path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if path_str.contains("Ctx") {
            return true;
        }
    }
    false
}

/// Check if type is Opt<T> (optional parameter)
fn is_opt_type(ty: &Type) -> bool {
    get_type_name(ty).map(|n| n == "Opt").unwrap_or(false)
}

/// Check if type is Rest<T> (variadic parameter)
fn is_rest_type(ty: &Type) -> bool {
    get_type_name(ty).map(|n| n == "Rest").unwrap_or(false)
}


// ============================================================================
// Type to TypeScript Conversion
// ============================================================================

/// Convert a Rust type to TypeScript type string
fn rust_type_to_typescript(ty: &Type, param_attrs: &[Attribute]) -> String {
    // Check for custom ts_type override
    if let Some(custom) = get_custom_ts_type(param_attrs) {
        return custom;
    }

    match ty {
        Type::Path(type_path) => {
            let type_name = type_path
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            match type_name.as_str() {
                // Primitive types
                "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "usize" | "isize" => {
                    "number".to_string()
                }
                "u64" | "i64" => "number".to_string(),
                "f32" | "f64" => "number".to_string(),
                "bool" => "boolean".to_string(),
                "String" => "string".to_string(),
                "str" => "string".to_string(),

                // Unit type
                "()" => "void".to_string(),

                // Option<T> -> T | null
                "Option" => {
                    if let Some(inner) = extract_inner_type(ty) {
                        format!("{} | null", rust_type_to_typescript(&inner, &[]))
                    } else {
                        "unknown | null".to_string()
                    }
                }

                // Vec<T> -> T[]
                "Vec" => {
                    if let Some(inner) = extract_inner_type(ty) {
                        // Special case: Vec<String> for args
                        let inner_ts = rust_type_to_typescript(&inner, &[]);
                        format!("{}[]", inner_ts)
                    } else {
                        "unknown[]".to_string()
                    }
                }

                // Opt<T> -> extract inner type (optionality handled at param level)
                "Opt" => {
                    if let Some(inner) = extract_inner_type(ty) {
                        rust_type_to_typescript(&inner, &[])
                    } else {
                        "unknown".to_string()
                    }
                }

                // Rest<T> -> extract inner type (variadic handled at param level)
                "Rest" => {
                    if let Some(inner) = extract_inner_type(ty) {
                        rust_type_to_typescript(&inner, &[])
                    } else {
                        "unknown".to_string()
                    }
                }

                // Result<T> -> extract inner type
                "Result" => {
                    if let Some(inner) = extract_inner_type(ty) {
                        rust_type_to_typescript(&inner, &[])
                    } else {
                        "unknown".to_string()
                    }
                }

                // QuickJS types
                "Value" => "unknown".to_string(),
                "Object" => "Record<string, unknown>".to_string(),

                // HashMap -> Record<string, T>
                "HashMap" => "Record<string, unknown>".to_string(),

                // Known API types
                "BufferInfo" | "CursorInfo" | "ViewportInfo" | "SpawnResult"
                | "BackgroundProcessResult" | "DirEntry" | "FileStat"
                | "CreateVirtualBufferResult" | "PromptSuggestion" | "TextPropertyEntry" => {
                    type_name
                }

                // Default: use type name as-is
                _ => type_name,
            }
        }
        Type::Tuple(tuple) if tuple.elems.is_empty() => "void".to_string(),
        Type::Reference(reference) => rust_type_to_typescript(&reference.elem, param_attrs),
        _ => "unknown".to_string(),
    }
}

// ============================================================================
// Method Parsing
// ============================================================================

/// Parse a method from an impl block
fn parse_method(method: &ImplItemFn) -> Option<ApiMethod> {
    // Skip methods marked with #[plugin_api(skip)]
    if should_skip(&method.attrs) {
        return None;
    }

    let rust_name = method.sig.ident.to_string();
    let doc_comment = extract_doc_comment(&method.attrs);

    // Determine if this is an async method - based ONLY on explicit attributes
    let explicit_async_promise = is_async_promise(&method.attrs);
    let explicit_async_thenable = is_async_thenable(&method.attrs);

    // Get custom JS name from attribute if specified, or convert from snake_case
    let custom_js_name = get_custom_js_name(&method.attrs);

    let (kind, js_name) = if explicit_async_thenable {
        // Explicit async thenable (cancellable)
        let name = custom_js_name.unwrap_or_else(|| to_camel_case(&rust_name));
        (ApiKind::AsyncThenable, name)
    } else if explicit_async_promise {
        // Explicit async promise
        let name = custom_js_name.unwrap_or_else(|| to_camel_case(&rust_name));
        (ApiKind::AsyncPromise, name)
    } else {
        // Sync method - skip if starts with _ (internal methods)
        let name = custom_js_name.unwrap_or_else(|| to_camel_case(&rust_name));
        if name.starts_with('_') {
            return None; // Skip internal methods like _deleteThemeSync
        }
        (ApiKind::Sync, name)
    };

    // Parse parameters
    let mut params = Vec::new();
    for arg in &method.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            // Skip &self, &mut self
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();

                // Skip 'self' parameter
                if param_name == "self" {
                    continue;
                }

                let ty = &*pat_type.ty;

                // Skip rquickjs::Ctx parameter
                if is_quickjs_ctx(ty) {
                    continue;
                }

                let is_optional = is_opt_type(ty);
                let is_variadic = is_rest_type(ty);

                let ts_type = rust_type_to_typescript(ty, &pat_type.attrs);

                params.push(ParamInfo {
                    name: to_camel_case(&param_name),
                    ts_type,
                    is_optional,
                    is_variadic,
                });
            }
        }
    }

    // Parse return type
    let return_type = match &method.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some((**ty).clone()),
    };

    // Get TypeScript return type
    let ts_return_type = if let Some(custom) = get_custom_ts_return(&method.attrs) {
        custom
    } else if let Some(ref ty) = return_type {
        rust_type_to_typescript(ty, &method.attrs)
    } else {
        "void".to_string()
    };

    Some(ApiMethod {
        js_name,
        kind,
        params,
        ts_return_type,
        doc_comment,
    })
}

// ============================================================================
// TypeScript Generation
// ============================================================================

/// Generate TypeScript method signature
fn generate_ts_method(method: &ApiMethod) -> String {
    let mut lines = Vec::new();

    // Add doc comment
    if !method.doc_comment.is_empty() {
        lines.push("  /**".to_string());
        for line in method.doc_comment.lines() {
            lines.push(format!("   * {}", line));
        }
        lines.push("   */".to_string());
    }

    // Build parameter list
    let params: Vec<String> = method
        .params
        .iter()
        .map(|p| {
            if p.is_variadic {
                format!("...{}: {}[]", p.name, p.ts_type)
            } else if p.is_optional {
                format!("{}?: {}", p.name, p.ts_type)
            } else {
                format!("{}: {}", p.name, p.ts_type)
            }
        })
        .collect();

    // Build return type based on method kind
    let return_type = match &method.kind {
        ApiKind::Sync => method.ts_return_type.clone(),
        ApiKind::AsyncPromise => format!("Promise<{}>", method.ts_return_type),
        ApiKind::AsyncThenable => format!("ProcessHandle<{}>", method.ts_return_type),
    };

    lines.push(format!(
        "  {}({}): {};",
        method.js_name,
        params.join(", "),
        return_type
    ));

    lines.join("\n")
}

/// Generate the full TypeScript header and type definitions
fn generate_ts_header() -> String {
    r#"/**
 * Fresh Editor TypeScript Plugin API
 *
 * This file provides type definitions for the Fresh editor's TypeScript plugin system.
 * Plugins have access to the global `editor` object which provides methods to:
 * - Query editor state (buffers, cursors, viewports)
 * - Modify buffer content (insert, delete text)
 * - Add visual decorations (overlays, highlighting)
 * - Interact with the editor UI (status messages, prompts)
 *
 * AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
 * Generated by fresh-plugin-api-macros from JsEditorApi impl
 */

/**
 * Get the editor API instance.
 * Plugins must call this at the top of their file to get a scoped editor object.
 */
declare function getEditor(): EditorAPI;

/** Handle for a cancellable async operation */
interface ProcessHandle<T> extends PromiseLike<T> {
  /** Promise that resolves to the result when complete */
  readonly result: Promise<T>;
  /** Cancel/kill the operation. Returns true if cancelled, false if already completed */
  kill(): Promise<boolean>;
}

/** Buffer identifier */
type BufferId = number;

/** Split identifier */
type SplitId = number;

/** Buffer information */
interface BufferInfo {
  id: number;
  path: string;
  modified: boolean;
  length: number;
}

/** Cursor information with optional selection */
interface CursorInfo {
  position: number;
  selection?: { start: number; end: number } | null;
}

/** Viewport information */
interface ViewportInfo {
  top_byte: number;
  left_column: number;
  width: number;
  height: number;
}

/** Result from spawnProcess */
interface SpawnResult {
  stdout: string;
  stderr: string;
  exit_code: number;
}

/** Result from spawnBackgroundProcess */
interface BackgroundProcessResult {
  process_id: number;
}

/** Directory entry */
interface DirEntry {
  name: string;
  is_file: boolean;
  is_dir: boolean;
}

/** File stat information */
interface FileStat {
  exists: boolean;
  is_file: boolean;
  is_dir: boolean;
  size: number;
  readonly: boolean;
}

/** Prompt suggestion */
interface PromptSuggestion {
  text: string;
  description?: string | null;
  value?: string | null;
  disabled?: boolean | null;
  keybinding?: string | null;
}

/** Text property entry for virtual buffers */
interface TextPropertyEntry {
  text: string;
  properties: Record<string, unknown>;
}

/** Result from createVirtualBufferInSplit */
interface CreateVirtualBufferResult {
  buffer_id: number;
  split_id?: number | null;
}

"#
    .to_string()
}

// ============================================================================
// Main Proc Macro
// ============================================================================

/// Generate TypeScript definitions from a JsEditorApi impl block
///
/// This macro parses the impl block and generates:
/// - `JSEDITORAPI_TYPESCRIPT_DEFINITIONS: &str` - Full .d.ts content
/// - `JSEDITORAPI_JS_METHODS: &[&str]` - List of all JS method names
///
/// # Attributes
///
/// - `#[plugin_api(skip)]` - Don't expose this method to TypeScript
/// - `#[plugin_api(js_name = "...")]` - Custom JS method name
/// - `#[plugin_api(async_promise)]` - Mark as async returning Promise<T>
/// - `#[plugin_api(async_thenable)]` - Mark as async returning ProcessHandle<T>
/// - `#[plugin_api(ts_type = "...")]` - Custom TypeScript type for param
/// - `#[plugin_api(ts_return = "...")]` - Custom TypeScript return type
#[proc_macro_attribute]
pub fn plugin_api_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    // Get the impl target name (e.g., JsEditorApi)
    let impl_name = if let Type::Path(type_path) = &*input.self_ty {
        type_path
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    } else {
        "Unknown".to_string()
    };

    let ts_const_name = format_ident!("{}_TYPESCRIPT_DEFINITIONS", impl_name.to_uppercase());
    let methods_const_name = format_ident!("{}_JS_METHODS", impl_name.to_uppercase());

    // Parse all methods
    let methods: Vec<ApiMethod> = input
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                parse_method(method)
            } else {
                None
            }
        })
        .collect();

    // Generate TypeScript definitions
    let ts_header = generate_ts_header();
    let ts_methods: Vec<String> = methods.iter().map(generate_ts_method).collect();

    let ts_interface = format!(
        "{}/**\n * Main editor API interface\n */\ninterface EditorAPI {{\n{}\n}}\n",
        ts_header,
        ts_methods.join("\n\n")
    );

    // Collect JS method names
    let js_method_names: Vec<String> = methods.iter().map(|m| m.js_name.clone()).collect();

    // Write TypeScript file if CARGO_MANIFEST_DIR is set (means we're building the main crate)
    // Only write during compilation of the crate that uses this macro, not during macro crate build
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        // Check if this is the fresh-editor crate (not the macro crate)
        if manifest_dir.ends_with("fresh") || manifest_dir.contains("fresh-editor") {
            let ts_path = std::path::Path::new(&manifest_dir)
                .join("plugins")
                .join("lib")
                .join("fresh.d.ts");

            // Only write if the file content would change (to avoid unnecessary rebuilds)
            let should_write = match std::fs::read_to_string(&ts_path) {
                Ok(existing) => existing != ts_interface,
                Err(_) => true, // File doesn't exist, write it
            };

            if should_write {
                // Ensure directory exists
                if let Some(parent) = ts_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&ts_path, &ts_interface);
            }
        }
    }

    // Generate the output - preserve the original impl and add constants
    let expanded = quote! {
        // Original impl block (preserved as-is)
        #input

        /// TypeScript definitions for the plugin API
        ///
        /// This constant contains the full .d.ts content that should be written
        /// to `plugins/lib/fresh.d.ts`.
        pub const #ts_const_name: &str = #ts_interface;

        /// List of all JavaScript method names from the API
        ///
        /// Use this to verify that all methods are correctly exposed.
        pub const #methods_const_name: &[&str] = &[#(#js_method_names),*];
    };

    TokenStream::from(expanded)
}

/// Marker attribute for API method customization
///
/// Usage:
/// - `#[plugin_api(skip)]` - Don't expose this method to JS
/// - `#[plugin_api(js_name = "customName")]` - Use a custom JS method name
/// - `#[plugin_api(async_promise)]` - Mark as async returning Promise<T>
/// - `#[plugin_api(async_thenable)]` - Mark as async returning ProcessHandle<T>
/// - `#[plugin_api(ts_type = "TypeName")]` - Custom TypeScript type for parameter
/// - `#[plugin_api(ts_return = "TypeName")]` - Custom TypeScript return type
#[proc_macro_attribute]
pub fn plugin_api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This is a marker attribute, just pass through the item
    item
}
