//! TypeScript type generation using ts-rs
//!
//! This module collects all API types with `#[derive(TS)]` and generates
//! TypeScript declarations that are combined with the proc macro output.
//! The generated TypeScript is validated and formatted using oxc.

use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use ts_rs::TS;

use crate::services::plugins::api::{
    ActionPopupAction, ActionSpec, BufferInfo, BufferSavedDiff, CompositeHunk,
    CompositeLayoutConfig, CompositePaneStyle, CompositeSourceConfig, CursorInfo, LayoutHints,
    TsHighlightSpan, ViewTokenStyle, ViewTokenWire, ViewTokenWireKind, ViewportInfo,
};

/// Collect all ts-rs type declarations into a single string
pub fn collect_ts_types() -> String {
    let mut types = Vec::new();

    // Core types used in EditorAPI
    types.push(BufferInfo::decl());
    types.push(CursorInfo::decl());
    types.push(ViewportInfo::decl());
    types.push(ActionSpec::decl());
    types.push(BufferSavedDiff::decl());
    types.push(LayoutHints::decl());

    // Composite buffer types
    types.push(CompositeLayoutConfig::decl());
    types.push(CompositeSourceConfig::decl());
    types.push(CompositePaneStyle::decl());
    types.push(CompositeHunk::decl());

    // View transform types
    types.push(ViewTokenWireKind::decl());
    types.push(ViewTokenStyle::decl());
    types.push(ViewTokenWire::decl());

    // UI types
    types.push(ActionPopupAction::decl());
    types.push(TsHighlightSpan::decl());

    types.join("\n\n")
}

/// Validate TypeScript syntax using oxc parser
///
/// Returns Ok(()) if the syntax is valid, or an error with the parse errors.
pub fn validate_typescript(source: &str) -> Result<(), String> {
    let allocator = Allocator::default();
    let source_type = SourceType::d_ts();

    let parser_ret = Parser::new(&allocator, source, source_type).parse();

    if parser_ret.errors.is_empty() {
        Ok(())
    } else {
        let errors: Vec<String> = parser_ret.errors.iter().map(|e| e.to_string()).collect();
        Err(format!("TypeScript parse errors:\n{}", errors.join("\n")))
    }
}

/// Format TypeScript source code using oxc codegen
///
/// Parses the TypeScript and regenerates it with consistent formatting.
/// Returns the original source if parsing fails.
pub fn format_typescript(source: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::d_ts();

    let parser_ret = Parser::new(&allocator, source, source_type).parse();

    if !parser_ret.errors.is_empty() {
        // Return original source if parsing fails
        return source.to_string();
    }

    // Generate formatted code from AST
    Codegen::new().build(&parser_ret.program).code
}

/// Generate and write the complete fresh.d.ts file
///
/// Combines ts-rs generated types with proc macro output,
/// validates the syntax, formats the output, and writes to disk.
pub fn write_fresh_dts() -> Result<(), String> {
    use crate::services::plugins::backend::quickjs_backend::{
        JSEDITORAPI_TS_EDITOR_API, JSEDITORAPI_TS_PREAMBLE,
    };

    let ts_types = collect_ts_types();

    let content = format!(
        "{}\n{}\n{}",
        JSEDITORAPI_TS_PREAMBLE, ts_types, JSEDITORAPI_TS_EDITOR_API
    );

    // Validate the generated TypeScript syntax
    validate_typescript(&content)?;

    // Format the TypeScript
    let formatted = format_typescript(&content);

    // Determine output path
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let output_path = std::path::Path::new(&manifest_dir)
        .join("plugins")
        .join("lib")
        .join("fresh.d.ts");

    // Only write if content changed
    let should_write = match std::fs::read_to_string(&output_path) {
        Ok(existing) => existing != formatted,
        Err(_) => true,
    };

    if should_write {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&output_path, &formatted).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate, validate, format, and write fresh.d.ts
    /// Run with: cargo test --features plugins write_fresh_dts_file -- --ignored --nocapture
    #[test]
    #[ignore]
    fn write_fresh_dts_file() {
        // write_fresh_dts validates syntax and formats before writing
        write_fresh_dts().expect("Failed to write fresh.d.ts");
        println!("Successfully generated, validated, and formatted fresh.d.ts");
    }
}
