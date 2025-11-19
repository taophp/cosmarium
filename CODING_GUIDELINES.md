# Cosmarium Coding Guidelines

This document outlines the coding standards and best practices for the Cosmarium project. These guidelines apply to both human developers and AI assistants.

## 1. General Philosophy

- **Modularity**: Respect the plugin architecture. Core functionality belongs in `cosmarium-core`, specific features in plugins.
- **Safety**: Leverage Rust's type system to ensure memory safety and concurrency.
- **Performance**: Write efficient code, avoiding unnecessary allocations and blocking operations in the UI thread.
- **Clarity**: Code should be readable and self-documenting.

## 2. Rust Style Guide

We follow the standard Rust style guidelines.

- **Formatting**: All code must be formatted with `cargo fmt`.
- **Linting**: Code must pass `cargo clippy` with no warnings.
- **Naming**:
    - `UpperCamelCase` for types (structs, enums, traits).
    - `snake_case` for functions, methods, variables, and modules.
    - `SCREAMING_SNAKE_CASE` for constants and statics.

## 3. Architecture & Project Structure

- **cosmarium-core**: Contains the foundational traits, event bus, and application state. Minimal dependencies.
- **cosmarium-plugin-api**: Defines the contract between the core and plugins. Must remain stable.
- **cosmarium-plugins**: Collection of built-in plugins. Each plugin should be a separate crate or module.
- **cosmarium-app**: The executable entry point. Handles UI initialization and plugin loading.

## 4. Error Handling

- **Libraries (Core, Plugins)**: Use `thiserror` to define custom error types. This allows consumers to handle specific errors programmatically.
- **Applications (App)**: Use `anyhow` for easy error propagation in the top-level application logic.
- **Panics**: Avoid `unwrap()` and `expect()` in production code. Handle errors gracefully.

## 5. Testing

- **Unit Tests**: Every module should have a `tests` module for unit tests.
- **Integration Tests**: Use the `tests/` directory for testing interactions between components.
- **Doc Tests**: Public functions should have documentation examples that are also tests.

## 6. Documentation

- **Public API**: All public structs, enums, traits, and functions must have doc comments (`///`).
- **Modules**: Top-level modules should have a description (`//!`).
- **Comments**: Use comments to explain *why* something is done, not *what* is done (the code should show that).

## 7. Version Control

- **Commits**: Use conventional commits (e.g., `feat:`, `fix:`, `docs:`, `refactor:`).
- **Branches**: Use feature branches for new development.

## 8. AI Assistant Instructions

- When generating code, always check for existing patterns in the codebase.
- Prioritize safety and correctness over brevity.
- Explain complex logic in comments.
