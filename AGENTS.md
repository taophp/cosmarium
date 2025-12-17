AGENTS - Repo guidelines for agentic editing

Build & Run
- Build (dev): `cargo build --verbose`
- Build (release): `cargo build --release`

Format & Lint
- Format: `cargo fmt --all` (CI uses `cargo fmt --all -- --check`)
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`

Tests
- Run all tests: `cargo test --verbose`
- Run a single unit test (filter): `cargo test <TEST_NAME> -- --nocapture` (substring filter)
- Run an exact single test: `cargo test <TEST_NAME> -- --exact --nocapture`
- Run an integration test file: `cargo test --test <test_filename>`

Code Style
- Formatting: always run `cargo fmt`; follow rustfmt defaults
- Imports: explicit imports; group order: `std` -> external crates -> workspace crates -> local modules; avoid glob (`*`) imports
- Naming: `UpperCamelCase` for types, `snake_case` for functions/variables/modules, `SCREAMING_SNAKE_CASE` for consts
- Types: prefer concrete types when clear, use traits in public APIs where appropriate

Error Handling
- Libraries/core/plugins: define errors with `thiserror` and return `Result<_, E>`
- Applications/binaries: use `anyhow` for top-level error propagation
- Avoid `unwrap()`/`expect()` in production code; prefer graceful error handling

Repo notes
- CI installs `rustfmt` and `clippy` and runs format/lint/build/test (see `.github/workflows/ci.yml`).
- No Cursor rules or Copilot instruction files were found (`.cursor/`, `.cursorrules`, `.github/copilot-instructions.md`).
- See `CODING_GUIDELINES.md` for extended architecture and conventions.
