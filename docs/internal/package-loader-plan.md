# Move Package Loading from pkg Plugin to Rust

## Problem

The `pkg` plugin's startup `loadInstalledPackages()` function loads language
packs and bundles via async JS callbacks. This serializes grammar rebuilds
behind `await reloadGrammars()` calls, causing cascading background builds
that take ~90s under constrained CPU (3 builds instead of 1).

The root cause is an architectural split: Rust loads only grammars from
`languages/packages/`, while the pkg plugin loads everything else (language
config, LSP config, bundle grammars, bundle plugins). The pkg plugin
exists because Rust was never taught to read those manifest fields.

## Goal

Move all "load installed packages at startup" logic into Rust. The pkg
plugin becomes install/uninstall/browse only — no startup loading.

This eliminates all async grammar rebuilds from plugin callbacks. The
background grammar build happens once, including all grammar files from
both `languages/packages/` and `bundles/packages/`.

## Current State

### What Rust handles at startup
- Grammars from `~/.config/fresh/languages/packages/` (via `load_language_pack_grammars` in `loader.rs`)
- Plugin discovery from `~/.config/fresh/plugins/packages/*/`
- Parses a minimal `FreshPackageManifest` (only `name` + `fresh.grammar`)

### What the pkg plugin handles at startup (`loadInstalledPackages`)
- Language packs (`languages/packages/`): registers grammar, language config, LSP config via JS API, then `await reloadGrammars()`
- Bundles (`bundles/packages/`): registers grammars + language config + LSP config for each language entry, loads bundle plugins via `await loadPlugin()`, reloads themes

### Rust types that already exist
- `LanguageConfig` in `config.rs` — has `comment_prefix`, `tab_size`, `use_tabs`, `auto_indent`, `formatter`, etc.
- `LspServerConfig` in `types.rs` — has `command`, `args`, `auto_start`, `initialization_options`
- `FreshPackageManifest` in `loader.rs` — only parses `name` + `fresh.grammar` (needs expansion)

### Schema
- `plugins/schemas/package.schema.json` — hand-maintained JSON schema covering all package types
- CONTRIBUTING.md line 50: "Package schema: Manually maintained"

### Recent changes on master (parallel plugin loading)
- `7a63ee07` — Plugin loading is now two-phase: Phase 1 reads files and
  transpiles TS→JS in parallel using `std::thread::scope`; Phase 2 executes
  prepared JS serially in QuickJS in topologically-sorted order.
- `faff0a47` — Plugins can declare dependencies via `import type { T } from
  "fresh:plugin/name"`. Dependencies are extracted during Phase 1 and used
  for topological sorting in Phase 2.
- Bundle plugin dirs added to the `plugin_dirs` list will automatically get
  the parallel prepare → serial execute treatment. No special handling needed.
- The `LoadPlugin` request (used by JS `editor.loadPlugin()` for dynamic
  single-plugin loads) still uses the old serial path. Moving bundle plugins
  to Rust plugin dirs means they use the faster parallel path instead.

## Design

### 1. Define `PackageManifest` Rust struct with `#[derive(Deserialize, JsonSchema)]`

A single serde struct matching the full `package.schema.json` schema. Lives in
a new module `crates/fresh-editor/src/services/packages.rs` (or
`services/packages/manifest.rs` if it grows).

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "type")]
    pub package_type: PackageType,
    #[serde(default)]
    pub fresh: Option<FreshManifestConfig>,
    // author, license, repository, keywords — not needed at load time,
    // but included for schema generation
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PackageType {
    Plugin,
    Theme,
    ThemePack,
    Language,
    Bundle,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FreshManifestConfig {
    pub grammar: Option<GrammarConfig>,
    pub language: Option<LanguagePackConfig>,
    pub lsp: Option<LspPackConfig>,
    pub languages: Option<Vec<BundleLanguage>>,  // bundles
    pub plugins: Option<Vec<BundlePlugin>>,      // bundles
    pub themes: Option<Vec<BundleTheme>>,         // bundles
    pub entry: Option<String>,                    // plugins
    // ...
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BundleLanguage {
    pub id: String,
    pub grammar: Option<GrammarConfig>,
    pub language: Option<LanguagePackConfig>,
    pub lsp: Option<LspPackConfig>,
}
```

The `LanguagePackConfig` and `LspPackConfig` use camelCase serde rename to
match the JSON schema (`commentPrefix`, `autoStart`, etc.), then convert to
the existing Rust `LanguageConfig` / `LspServerConfig` types.

### 2. Generate `package.schema.json` from Rust

Add a `"package"` arm to `src/bin/generate_schema.rs`:

```rust
"package" => {
    let schema = schema_for!(PackageManifest);
    serde_json::to_value(&schema).expect("...")
}
```

Update `scripts/gen_schema.sh` to also generate the package schema. Update
CONTRIBUTING.md to remove the "manually maintained" note.

### 3. Package scanner: `scan_installed_packages()`

A function (in the new module) that runs during `Editor::new()`, **before**
plugin loading:

```rust
pub struct PackageScanResult {
    /// Language configs to merge into Config.languages
    pub language_configs: Vec<(String, LanguageConfig)>,
    /// LSP configs to apply
    pub lsp_configs: Vec<(String, LspServerConfig)>,
    /// Additional grammar files for the background build
    /// (bundle grammars not already in languages/packages/)
    pub additional_grammars: Vec<(String, PathBuf, Vec<String>)>,
    /// Bundle plugin directories to add to the plugin loading list
    pub bundle_plugin_dirs: Vec<PathBuf>,
    /// Bundle theme directories for theme reloading
    pub bundle_theme_files: Vec<(PathBuf, String)>,
}

pub fn scan_installed_packages(config_dir: &Path) -> PackageScanResult {
    let mut result = PackageScanResult::default();

    // Scan languages/packages/
    scan_directory(config_dir.join("languages/packages"), |manifest, pkg_dir| {
        // For type: "language", extract language config + LSP config
        // Grammar loading is already handled by the grammar loader
    });

    // Scan bundles/packages/
    scan_directory(config_dir.join("bundles/packages"), |manifest, pkg_dir| {
        // For type: "bundle":
        //   - Extract language config + LSP config for each language entry
        //   - Collect grammar paths for additional_grammars
        //   - Collect plugin entry paths for bundle_plugin_dirs
        //   - Collect theme files for bundle_theme_files
    });

    result
}
```

### 4. Integrate into `Editor::new()` (in `with_options`)

Insert the scan between config creation and plugin loading (~line 1210):

```rust
// Scan installed packages (language packs + bundles)
let scan_result = packages::scan_installed_packages(&dir_context.config_dir);

// Apply language configs
for (lang_id, lang_config) in scan_result.language_configs {
    config.languages.entry(lang_id).or_default().merge_from(lang_config);
}

// Apply LSP configs
for (lang_id, lsp_config) in scan_result.lsp_configs {
    config.lsp.insert(lang_id, lsp_config);
    lsp.set_language_config(lang_id, lsp_config);
}

// Add bundle plugin dirs to the plugin loading list
for dir in scan_result.bundle_plugin_dirs {
    plugin_dirs.push(dir);
}

// Store additional grammars for the deferred background build
// (these get passed to start_background_grammar_build via flush_pending_grammars)
```

Bundle plugin dirs are added to `plugin_dirs` before the plugin loading loop.
This means bundle plugins go through the same parallel prepare → serial
execute pipeline (from `7a63ee07`) as all other plugins. They benefit from
parallel I/O and transpilation, participate in dependency-based topological
sorting (from `faff0a47`), and have first-writer-wins collision detection
(from `26a03625`).

This is strictly better than the current `editor.loadPlugin()` path, which
loads each bundle plugin serially via a one-off `LoadPlugin` request during
JS callback resolution — bypassing parallel preparation and dependency
ordering entirely.

### 5. Include bundle grammars in the background grammar build

The grammar loader (`for_editor` / `load`) already scans `languages/packages/`
for grammar files. Extend it to also scan `bundles/packages/*/` for grammar
files in the `fresh.languages[].grammar` entries. This way all grammars are
built in a single `builder.build()` pass.

Alternatively, pass `scan_result.additional_grammars` to the deferred
`start_background_grammar_build` so bundle grammars are included in the
initial build alongside any plugin-registered grammars from the first
event-loop tick.

The second option is simpler because the grammar loader doesn't need to learn
about bundles — we just feed it the paths. But the first is cleaner because
it means zero grammar rebuilds from plugin callbacks (the initial
`for_editor()` build has everything).

Preferred: extend the grammar loader to also scan `bundles/packages/`. This
uses the same `FreshPackageManifest` struct (step 1) to find grammar files
in `fresh.languages[].grammar`. The `load_language_pack_grammars` function
in `loader.rs` already takes a `GrammarLoader` trait, and the existing
`LocalGrammarLoader` can grow a `bundles_packages_dir()` method.

### 6. Remove `loadInstalledPackages()` from pkg plugin

Delete the startup IIFE at the bottom of `pkg.ts` (lines 3042-3066). The
`loadLanguagePack()` and `loadBundle()` functions stay — they're still
needed for dynamic install (when the user installs a package at runtime via
the package manager UI).

### 7. Update CONTRIBUTING.md

Change line 50 from:
```
- **Package schema** (`plugins/schemas/package.schema.json`): Manually maintained
```
to:
```
- **Package schema** (`plugins/schemas/package.schema.json`): Auto-generated from Rust types. Run: `./scripts/gen_schema.sh`
```

## Ordering / Commits

1. **Add `PackageManifest` struct + schema generation** — new Rust types,
   regenerate `package.schema.json`, update CONTRIBUTING.md. No behavior change.

2. **Add `scan_installed_packages()`** — new function, not yet called. Unit
   tests with mock package directories.

3. **Integrate scan into `Editor::new()`** — apply language/LSP configs,
   add bundle plugin dirs, pass bundle grammars to background build.

4. **Remove `loadInstalledPackages()` from pkg plugin** — the startup
   loader is now dead code.

5. **Extend grammar loader to scan `bundles/packages/`** (optional) — if we
   want all grammars in the initial `for_editor()` build rather than going
   through the "additional grammars" path.

## What Stays in the pkg Plugin

- `pkg_list` / `pkg_install_url` commands (install, uninstall, browse)
- Registry sync and search
- Lockfile management
- `loadLanguagePack()` / `loadBundle()` for **runtime** install (not startup)
- Package validation

## Risks

- **Manifest compatibility**: The Rust struct must parse all existing
  `package.json` files without error. Use `#[serde(default)]` liberally and
  test against real installed packages.

- **Ordering**: Language/LSP configs must be applied before plugins load, so
  plugins that query language config during init see the right values. The
  scan runs before the plugin loading loop, so this is satisfied.

- **Bundle plugin loading**: Bundle plugins currently load via
  `editor.loadPlugin()` in JS, which uses the serial `LoadPlugin` request —
  bypassing parallel preparation, dependency sorting, and collision detection.
  Moving them to the Rust `plugin_dirs` list means they go through the same
  two-phase parallel pipeline as all other plugins. This is strictly better:
  faster (parallel I/O and transpilation), correct ordering (topological sort
  respects their dependencies), and safer (first-writer-wins collision
  detection applies).

- **Themes from bundles**: Currently `loadBundle()` calls
  `editor.reloadThemes()`. The Rust side would need to load theme files from
  bundle paths during theme registry initialization. This is a minor addition
  to the theme loader.

- **Bundle plugins with dependencies on embedded plugins**: If a bundle
  plugin imports from an embedded plugin (e.g., `import type { T } from
  "fresh:plugin/some-embedded"`), the dependency system already handles this —
  topological sort works across all plugin directories. The only requirement
  is that the embedded plugin is in the same `PreparedPlugin` set, which it
  will be since all dirs are prepared together in Phase 1.
