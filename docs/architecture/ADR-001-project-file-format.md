# ADR-001: Project File Format and Versioning

## Status
Proposed

## Context
Cosmarium requires a robust project format that supports:
1.  **Metadata storage**: Configuration for the core application and plugins.
2.  **Content storage**: The actual creative writing content (documents).
3.  **Versioning**: Ability to track changes, branch, and collaborate.
4.  **AI-Friendliness**: Metadata formats should be optimized for LLM processing (token efficiency).

Currently, the application uses an ad-hoc JSON-based format which is not formally defined.

## Decision

We will adopt a **Folder-Based Project Structure** with **TOON (Token Oriented Object Notation)** for metadata and **Git** for version control.

### 1. Folder Structure
A project will be a directory containing:

```
ProjectName/
├── .git/               # Git repository for versioning
├── meta/               # Metadata directory
│   ├── core.toon       # Core project configuration (name, author, version)
│   ├── plugin1.toon    # Configuration/State for Plugin 1
│   └── ...
├── content/            # Content directory (documents)
│   ├── chapter1.md
│   └── ...
└── assets/             # (Optional) Images, reference materials
```

*Note: The user initially suggested `text.md` at the root. We propose a `content/` directory to better support multiple documents/chapters, which is a standard requirement for novel writing software.*

### 2. Metadata Format: TOON
We will use **TOON** (Token Oriented Object Notation) for all metadata files (`.toon`).
*   **Rationale**: TOON is designed to be token-efficient for LLMs, reducing context window usage and cost when AI agents interact with project metadata.
*   **Implementation**: We will use the `serde_toon2` crate for serialization/deserialization.

### 3. Versioning: Git
We will initialize a Git repository (`.git`) within each project folder.
*   **Rationale**: Provides industry-standard version control, enabling history, branching, and potential future collaboration features.
*   **Implementation**: We will use the **`gix`** crate (gitoxide).
    *   **Why `gix`?**: It is a pure Rust implementation, avoiding C bindings (`libgit2`). This is critical for **WASM support** (web target) and simplifies cross-compilation.
    *   **Capabilities**: `gix` supports all necessary high-level operations (init, commit, branch, status) required for our use case.

## Consequences

### Positive
*   **AI Optimization**: TOON reduces token usage for metadata.
*   **WASM Compatible**: Pure Rust stack (`gix`) ensures the core can compile to WASM for web deployment.
*   **Cross-Platform**: Easier cross-compilation without C dependencies.
*   **Robust Versioning**: Git provides powerful version control capabilities.

### Negative
*   **Dependencies**: Adds `serde_toon2` and `gix` to the dependency tree.
*   **Maturity**: `gix` is newer than `git2`, though feature-complete for our needs.
*   **Storage**: Git history will increase project size over time.

## Technical Implementation Details

1.  **Dependencies**:
    *   Add `serde_toon2 = "0.1.0"` to `cosmarium-core`.
    *   Add `gix = { version = "0.63", features = ["max"] }` to `cosmarium-core`.

2.  **Core Changes**:
    *   Update `Project` struct to load/save from the new structure.
    *   Implement `GitIntegration` using `gix`.
    *   Update `PluginManager` to read/write plugin-specific TOON files in `meta/`.

3.  **Migration**:
    *   Future work will be needed to migrate existing `project.json` projects to this new format.
