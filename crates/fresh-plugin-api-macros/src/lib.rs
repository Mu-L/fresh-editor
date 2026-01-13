//! Proc macros for type-safe plugin API bindings
//!
//! This crate provides the `#[plugin_api]` attribute macro that generates
//! QuickJS bindings from a trait definition.
//!
//! # Example
//!
//! ```rust,ignore
//! use fresh_plugin_api_macros::plugin_api;
//!
//! #[plugin_api]
//! pub trait EditorApi {
//!     /// Get the active buffer ID
//!     #[api(sync)]
//!     fn getActiveBufferId(&self) -> u32;
//!
//!     /// Spawn a process (async with thenable result)
//!     #[api(async_thenable)]
//!     fn spawnProcess(&self, command: String, args: Vec<String>, cwd: Option<String>) -> SpawnResult;
//!
//!     /// Delay execution
//!     #[api(async_simple)]
//!     fn delay(&self, ms: u32);
//! }
//! ```
//!
//! The macro generates:
//! - The trait definition itself
//! - A `register_<trait>_bindings` function that registers all methods with QuickJS
//! - JS wrapper code for async functions

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, FnArg, Ident, ItemTrait, Meta, Pat, ReturnType,
    TraitItem, Type,
};

/// API method kind
#[derive(Debug, Clone, PartialEq)]
enum ApiKind {
    /// Synchronous method - returns value directly
    Sync,
    /// Async method that returns a simple Promise<T>
    AsyncSimple,
    /// Async method that returns a thenable with .kill() support
    AsyncThenable,
    /// Internal method - not exposed to JS, used for wrappers
    Internal,
}

/// Parsed API method information
struct ApiMethod {
    name: Ident,
    js_name: String,
    kind: ApiKind,
    params: Vec<(Ident, Type)>,
    return_type: Option<Type>,
    doc_comment: String,
}

/// Parse the #[api(...)] attribute to determine the method kind
fn parse_api_attr(attrs: &[Attribute]) -> ApiKind {
    for attr in attrs {
        if attr.path().is_ident("api") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = meta_list.tokens.to_string();
                return match tokens.as_str() {
                    "sync" => ApiKind::Sync,
                    "async_simple" => ApiKind::AsyncSimple,
                    "async_thenable" => ApiKind::AsyncThenable,
                    "internal" => ApiKind::Internal,
                    _ => ApiKind::Sync, // default
                };
            }
        }
    }
    ApiKind::Sync // default
}

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

/// Parse a trait method into an ApiMethod
fn parse_method(item: &TraitItem) -> Option<ApiMethod> {
    if let TraitItem::Fn(method) = item {
        let name = method.sig.ident.clone();
        let js_name = name.to_string();
        let kind = parse_api_attr(&method.attrs);
        let doc_comment = extract_doc_comment(&method.attrs);

        // Parse parameters (skip &self)
        let params: Vec<(Ident, Type)> = method
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let FnArg::Typed(pat_type) = arg {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        return Some((pat_ident.ident.clone(), (*pat_type.ty).clone()));
                    }
                }
                None
            })
            .collect();

        // Parse return type
        let return_type = match &method.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => Some((**ty).clone()),
        };

        Some(ApiMethod {
            name,
            js_name,
            kind,
            params,
            return_type,
            doc_comment,
        })
    } else {
        None
    }
}

/// Generate QuickJS binding code for a sync method
fn generate_sync_binding(method: &ApiMethod, trait_name: &Ident) -> TokenStream2 {
    let js_name = &method.js_name;
    let rust_name = &method.name;

    let param_names: Vec<_> = method.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = method.params.iter().map(|(_, ty)| ty).collect();

    // Build the closure signature
    let closure_params = if param_names.is_empty() {
        quote! { || }
    } else {
        quote! { |#(#param_names: #param_types),*| }
    };

    // Return type handling
    let return_annotation = match &method.return_type {
        Some(ty) => quote! { -> #ty },
        None => quote! {},
    };

    quote! {
        {
            let impl_ref = impl_ref.clone();
            editor.set(#js_name, rquickjs::Function::new(ctx.clone(), move #closure_params #return_annotation {
                impl_ref.borrow().#rust_name(#(#param_names),*)
            })?)?;
        }
    }
}

/// Generate QuickJS binding code for an async_simple method
fn generate_async_simple_binding(method: &ApiMethod, _trait_name: &Ident) -> TokenStream2 {
    let js_name = &method.js_name;
    let internal_name = format!("_{js_name}Start");
    let rust_name = &method.name;

    let param_names: Vec<_> = method.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = method.params.iter().map(|(_, ty)| ty).collect();

    quote! {
        // Internal start function
        {
            let request_id = std::rc::Rc::clone(&next_request_id);
            let impl_ref = impl_ref.clone();
            editor.set(#internal_name, rquickjs::Function::new(ctx.clone(), move |#(#param_names: #param_types),*| -> u64 {
                let id = {
                    let mut id_ref = request_id.borrow_mut();
                    let id = *id_ref;
                    *id_ref += 1;
                    id
                };
                impl_ref.borrow().#rust_name(id, #(#param_names),*);
                id
            })?)?;
        }
        // JS wrapper will be added in bootstrap code
        _async_simple_methods.push(#js_name);
    }
}

/// Generate QuickJS binding code for an async_thenable method
fn generate_async_thenable_binding(method: &ApiMethod, _trait_name: &Ident) -> TokenStream2 {
    let js_name = &method.js_name;
    let internal_name = format!("_{js_name}Start");
    let rust_name = &method.name;

    let param_names: Vec<_> = method.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = method.params.iter().map(|(_, ty)| ty).collect();

    quote! {
        // Internal start function
        {
            let request_id = std::rc::Rc::clone(&next_request_id);
            let impl_ref = impl_ref.clone();
            editor.set(#internal_name, rquickjs::Function::new(ctx.clone(), move |#(#param_names: #param_types),*| -> u64 {
                let id = {
                    let mut id_ref = request_id.borrow_mut();
                    let id = *id_ref;
                    *id_ref += 1;
                    id
                };
                impl_ref.borrow().#rust_name(id, #(#param_names),*);
                id
            })?)?;
        }
        // JS wrapper will be added in bootstrap code
        _async_thenable_methods.push(#js_name);
    }
}

/// Main proc macro implementation
#[proc_macro_attribute]
pub fn plugin_api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemTrait);
    let trait_name = &input.ident;
    let register_fn_name = format_ident!("register_{}_bindings", trait_name.to_string().to_lowercase());

    // Parse all methods
    let methods: Vec<ApiMethod> = input
        .items
        .iter()
        .filter_map(parse_method)
        .filter(|m| m.kind != ApiKind::Internal)
        .collect();

    // Generate binding code for each method
    let bindings: Vec<TokenStream2> = methods
        .iter()
        .map(|method| match method.kind {
            ApiKind::Sync => generate_sync_binding(method, trait_name),
            ApiKind::AsyncSimple => generate_async_simple_binding(method, trait_name),
            ApiKind::AsyncThenable => generate_async_thenable_binding(method, trait_name),
            ApiKind::Internal => quote! {},
        })
        .collect();

    // Generate the JS wrapper code string for async methods
    let async_simple_wrappers: Vec<String> = methods
        .iter()
        .filter(|m| m.kind == ApiKind::AsyncSimple)
        .map(|m| format!(
            "_editorCore.{name} = _wrapAsync(_editorCore._{name}Start, \"{name}\");",
            name = m.js_name
        ))
        .collect();

    let async_thenable_wrappers: Vec<String> = methods
        .iter()
        .filter(|m| m.kind == ApiKind::AsyncThenable)
        .map(|m| format!(
            "_editorCore.{name} = _wrapAsyncThenable(_editorCore._{name}Start, \"{name}\");",
            name = m.js_name
        ))
        .collect();

    let js_wrappers = [async_simple_wrappers, async_thenable_wrappers].concat().join("\n                ");

    // Generate the output
    let expanded = quote! {
        // Original trait definition
        #input

        /// Register all API bindings with QuickJS
        ///
        /// This function is auto-generated by the #[plugin_api] macro.
        /// It registers all methods from the trait as JavaScript functions.
        pub fn #register_fn_name<T: #trait_name>(
            ctx: &rquickjs::Ctx<'_>,
            editor: &rquickjs::Object<'_>,
            impl_ref: std::rc::Rc<std::cell::RefCell<T>>,
            next_request_id: std::rc::Rc<std::cell::RefCell<u64>>,
        ) -> anyhow::Result<String> {
            // Track async methods for JS wrapper generation
            let mut _async_simple_methods: Vec<&str> = Vec::new();
            let mut _async_thenable_methods: Vec<&str> = Vec::new();

            #(#bindings)*

            // Return JS wrapper code to be executed in bootstrap
            Ok(#js_wrappers.to_string())
        }
    };

    TokenStream::from(expanded)
}

/// Attribute for marking individual API methods
///
/// Usage:
/// - `#[api(sync)]` - Synchronous method
/// - `#[api(async_simple)]` - Async returning Promise<T>
/// - `#[api(async_thenable)]` - Async returning thenable with .kill()
/// - `#[api(internal)]` - Not exposed to JS
#[proc_macro_attribute]
pub fn api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This is a marker attribute, just pass through the item
    item
}
