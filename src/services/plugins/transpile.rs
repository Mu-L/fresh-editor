//! TypeScript to JavaScript transpilation using oxc
//!
//! This module provides TypeScript transpilation without deno_ast,
//! using the oxc toolchain for parsing, transformation, and code generation.

use anyhow::{anyhow, Result};
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Transpile TypeScript source code to JavaScript
pub fn transpile_typescript(source: &str, filename: &str) -> Result<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();

    // Parse
    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if !parser_ret.errors.is_empty() {
        let errors: Vec<String> = parser_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(anyhow!("TypeScript parse errors: {}", errors.join("; ")));
    }

    let mut program = parser_ret.program;

    // Semantic analysis (required for transformer)
    let semantic_ret = SemanticBuilder::new().build(&program);

    if !semantic_ret.errors.is_empty() {
        let errors: Vec<String> = semantic_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(anyhow!("Semantic errors: {}", errors.join("; ")));
    }

    // Get scoping info for transformer
    let scoping = semantic_ret.semantic.into_scoping();

    // Transform (strip TypeScript types)
    let transform_options = TransformOptions::default();
    let transformer_ret = Transformer::new(
        &allocator,
        Path::new(filename),
        &transform_options,
    )
    .build_with_scoping(scoping, &mut program);

    if !transformer_ret.errors.is_empty() {
        let errors: Vec<String> = transformer_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(anyhow!("Transform errors: {}", errors.join("; ")));
    }

    // Generate JavaScript
    let codegen_ret = Codegen::new().build(&program);

    Ok(codegen_ret.code)
}

/// Check if source contains ES module imports (import ... from ...)
pub fn has_es_imports(source: &str) -> bool {
    // Simple heuristic: look for import statements
    // This catches: import X from "...", import { X } from "...", import * as X from "..."
    source.contains("import ") && source.contains(" from ")
}

/// Bundle a module and all its local imports into a single file
/// Only handles relative imports (./path or ../path), not npm packages
pub fn bundle_module(entry_path: &Path) -> Result<String> {
    let mut visited = HashSet::new();
    let mut output = String::new();
    bundle_recursive(entry_path, &mut visited, &mut output)?;
    Ok(output)
}

fn bundle_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    output: &mut String,
) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical.clone()) {
        return Ok(()); // Already bundled (circular import protection)
    }

    let source = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read {}: {}", path.display(), e))?;

    let imports = extract_local_imports(&source);
    let parent_dir = path.parent().unwrap_or(Path::new("."));

    // Resolve and bundle dependencies first
    for import_path in imports {
        let resolved = resolve_import(&import_path, parent_dir)?;
        bundle_recursive(&resolved, visited, output)?;
    }

    // Strip imports and append this module's code
    let stripped = strip_imports(&source);
    let filename = path.to_str().unwrap_or("unknown.ts");
    let transpiled = transpile_typescript(&stripped, filename)?;
    output.push_str(&transpiled);
    output.push('\n');

    Ok(())
}

/// Extract local relative imports from source
/// Only extracts imports starting with ./ or ../
fn extract_local_imports(source: &str) -> Vec<String> {
    let mut imports = Vec::new();

    // Match: import ... from "./..." or import ... from "../..."
    // Simple regex-like parsing without regex dependency
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with("import ") {
            continue;
        }

        // Find the 'from' part
        if let Some(from_idx) = line.find(" from ") {
            let after_from = &line[from_idx + 6..];
            // Extract the string between quotes
            let quote_char = if after_from.starts_with('"') {
                '"'
            } else if after_from.starts_with('\'') {
                '\''
            } else {
                continue;
            };

            if let Some(end_idx) = after_from[1..].find(quote_char) {
                let import_path = &after_from[1..end_idx + 1];
                // Only include local imports
                if import_path.starts_with("./") || import_path.starts_with("../") {
                    imports.push(import_path.to_string());
                }
            }
        }
    }

    imports
}

/// Resolve an import path relative to the importing file's directory
fn resolve_import(import_path: &str, parent_dir: &Path) -> Result<PathBuf> {
    let base = parent_dir.join(import_path);

    // Try various extensions
    if base.exists() {
        return Ok(base);
    }

    let with_ts = base.with_extension("ts");
    if with_ts.exists() {
        return Ok(with_ts);
    }

    let with_js = base.with_extension("js");
    if with_js.exists() {
        return Ok(with_js);
    }

    // Try index files
    let index_ts = base.join("index.ts");
    if index_ts.exists() {
        return Ok(index_ts);
    }

    let index_js = base.join("index.js");
    if index_js.exists() {
        return Ok(index_js);
    }

    Err(anyhow!("Cannot resolve import '{}' from {}", import_path, parent_dir.display()))
}

/// Strip import statements from source
fn strip_imports(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            // Remove import statements but keep other code
            !(trimmed.starts_with("import ") && trimmed.contains(" from "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_basic_typescript() {
        let source = r#"
            const x: number = 42;
            function greet(name: string): string {
                return `Hello, ${name}!`;
            }
        "#;

        let result = transpile_typescript(source, "test.ts").unwrap();
        assert!(result.contains("const x = 42"));
        assert!(result.contains("function greet(name)"));
        assert!(!result.contains(": number"));
        assert!(!result.contains(": string"));
    }

    #[test]
    fn test_transpile_interface() {
        let source = r#"
            interface User {
                name: string;
                age: number;
            }
            const user: User = { name: "Alice", age: 30 };
        "#;

        let result = transpile_typescript(source, "test.ts").unwrap();
        assert!(!result.contains("interface"));
        assert!(result.contains("const user = {"));
    }

    #[test]
    fn test_transpile_type_alias() {
        let source = r#"
            type ID = number | string;
            const id: ID = 123;
        "#;

        let result = transpile_typescript(source, "test.ts").unwrap();
        assert!(!result.contains("type ID"));
        assert!(result.contains("const id = 123"));
    }

    #[test]
    fn test_has_es_imports() {
        assert!(has_es_imports("import { foo } from './lib'"));
        assert!(has_es_imports("import foo from 'bar'"));
        assert!(!has_es_imports("const x = 1;"));
        assert!(!has_es_imports("// import foo from 'bar'")); // comment doesn't count as real import
    }

    #[test]
    fn test_extract_local_imports() {
        let source = r#"
            import { foo } from "./lib/utils";
            import bar from "../shared/bar";
            import external from "external-package";
            const x = 1;
        "#;

        let imports = extract_local_imports(source);
        assert_eq!(imports.len(), 2);
        assert!(imports.contains(&"./lib/utils".to_string()));
        assert!(imports.contains(&"../shared/bar".to_string()));
        // external-package should NOT be included
        assert!(!imports.iter().any(|i| i.contains("external")));
    }

    #[test]
    fn test_strip_imports() {
        let source = r#"import { foo } from "./lib";
import bar from "../bar";
const x = foo() + bar();"#;

        let stripped = strip_imports(source);
        assert!(!stripped.contains("import"));
        assert!(stripped.contains("const x = foo() + bar();"));
    }
}
