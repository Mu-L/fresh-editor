# Repository Guidelines

## Project Structure & Module Organization
- Core editor code: `src/` (app logic, rendering, input, model). Key entry points: `src/app/mod.rs`, `src/view/`.
- Tests: `tests/e2e/` for end-to-end coverage, `tests/common/` for harness/utilities, property tests under `tests/property_tests.rs`.
- Plugins and examples: `plugins/` (TypeScript plugins), `examples/` (sample configs and usage), `themes/` for color schemes, `keymaps/` for default keybindings.
- Build artifacts: `target/` (large), `dist-workspace.toml` and `build.rs` for build plumbing.

## Build, Test, and Development Commands
- `cargo build` — compile the project.
- `cargo test` — run all tests; heavy due to e2e/property suites.
- Targeted e2e: `cargo test --package fresh-editor e2e::<path>::<test_name> -- --nocapture` (e.g., `e2e::file_browser::test_file_browser_prompt_shows_buffer_directory`).
- Snapshot refresh (when appropriate): `cargo insta test` or `cargo insta accept` after verifying diffs.

## Coding Style & Naming Conventions
- Rust 2021 edition; follow `rustfmt` defaults (4-space indent). Keep code ASCII unless required by data.
- Prefer small, focused modules; place shared types in `src/app/types.rs` or relevant submodules.
- Use descriptive identifiers; file-backed buffers should display working-dir-relative names when possible.
- Comment sparingly to clarify non-obvious logic (e.g., platform-specific behavior, canonicalization).

## Testing Guidelines
- Primary framework: Rust `cargo test`, with `insta` snapshots for visuals and vt100 output.
- E2E harness lives in `tests/common/harness.rs`; prefer targeted tests for new UI flows in `tests/e2e/`.
- Name tests descriptively using snake_case; group by feature (e.g., `tests/e2e/merge_conflict.rs`).
- For expensive suites (property tests), run only when touching related logic.

## Commit & Pull Request Guidelines
- Use concise, imperative commit messages (e.g., “Make buffer display names working-dir relative”).
- Keep commits scoped and readable; include tests or rationale when behavior changes.
- PRs should explain the change, list key impacts, and note how it was tested (`cargo test ...`). Add screenshots or terminal output for UI/visual updates when relevant.

## Agent-Specific Tips
- Avoid large rebuilds when possible; re-use `cargo test --package fresh-editor <targeted>` during iteration.
- Mind platform differences: paths are canonicalized (e.g., `/var` vs `/private/var`), and macOS shows ⌘/⌥/⇧ in shortcut labels; write assertions tolerant of these differences.
