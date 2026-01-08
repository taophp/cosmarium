# Cosmarium

**Cosmarium** is a next-generation creative writing software designed for fiction authors (novels, short stories, novellas, long-form series) who want power, modularity, speed, and immersion.

Built in Rust with EGUI, Cosmarium puts modularity, ergonomics, and openness at the heart of its architecture through a comprehensive plugin system.

---

## ✨ Key Features

- **Modular Plugin Architecture**: Everything is a plugin - enable/disable features as needed
- **Immersive Markdown Editor**: Distraction-free writing with syntax highlighting and live preview
- **Cross-Platform**: Desktop (priority) and web support via EGUI
- **Real-time Statistics**: Word count, reading time, writing session analytics
- **Flexible Project Management**: Compressed files or directory structures
- **Modern UI**: Dark/light themes, customizable layouts, responsive design

---

## 🏗️ Architecture

Cosmarium follows a modular workspace structure:

```
cosmarium/
├── cosmarium-core/           # Core application logic
├── cosmarium-plugin-api/     # Plugin development API
├── cosmarium-plugins/        # Built-in plugins
│   └── markdown-editor/      # Reference implementation
├── cosmarium-app/            # Main application executable
└── Cargo.toml               # Workspace configuration
```

### Core Components

- **cosmarium-core**: Event system, plugin management, document/project handling
- **cosmarium-plugin-api**: Type-safe plugin API with traits and utilities  
- **cosmarium-plugins**: Extensible plugin collection
- **cosmarium-app**: EGUI-based desktop/web application

---

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+ with Cargo
- Git

### Building from Source

```bash
git clone https://github.com/cosmarium/cosmarium
cd cosmarium
cargo build --release
```

### Running

```bash
# Run the application
cargo run --bin cosmarium

# Run with specific project
cargo run --bin cosmarium -- --project /path/to/project.cosmarium

# Enable debug logging
cargo run --bin cosmarium -- --debug
```

### Development

```bash
# Check all packages
cargo check

# Run tests
cargo test

# Check specific package
cargo check -p cosmarium-core

# Build for web (requires wasm32 target)
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown -p cosmarium-app --features web
```

---

## 🔌 Plugin Development

Cosmarium's plugin system allows extending functionality through a type-safe API:

```rust
use cosmarium_plugin_api::{Plugin, PluginInfo, PanelPlugin, PluginContext};
use egui::Ui;

struct MyPlugin;

impl Plugin for MyPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new("my-plugin", "1.0.0", "Description", "Author")
    }
    
    fn initialize(&mut self, ctx: &mut PluginContext) -> anyhow::Result<()> {
        // Plugin initialization
        Ok(())
    }
}

impl PanelPlugin for MyPlugin {
    fn panel_title(&self) -> &str { "My Panel" }
    
    fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        ui.label("Hello from my plugin!");
    }
}
```

### Plugin Types

- **Panel Plugins**: UI panels (notes, entities, etc.)
- **Editor Plugins**: Text editing extensions  
- **Export Plugins**: Output format handlers
- **Analysis Plugins**: Writing statistics and feedback
- **AI Plugins**: Intelligent writing assistance

---

## 🗺️ Development Roadmap

### v0.1.0 – MVP Foundation ✅

- [x] Core plugin system architecture
- [x] Basic markdown editor with statistics
- [x] Project/document management
- [x] EGUI-based desktop application
- [x] Event-driven inter-plugin communication

### v0.2.0 – Enhanced Writing Experience
- [ ] Live markdown preview
- [ ] Advanced syntax highlighting
- [ ] Writing goals and session tracking
- [ ] Dialogue Assistance (Auto-replace `--` with em-dash `—`)
- [ ] Persistent Plugin Data (Atmosphere cache, user preferences)
- [ ] Multiple themes and customization
- [ ] Export to PDF/HTML/Word

### v0.3.0 – Smart Features
- [ ] AI writing assistance
- [ ] Style analysis (Hemingway-inspired)
- [ ] Entity extraction and management
- [ ] Timeline and plot structure tools

### v0.4.0 – Collaboration & Polish
- [ ] Real-time collaboration
- [ ] Version control integration
- [ ] Plugin marketplace
- [ ] Mobile/tablet support

---

## 🛠️ Technical Stack

- **Language**: Rust 2021 Edition
- **UI Framework**: EGUI (immediate mode GUI)
- **Async Runtime**: Tokio
- **Serialization**: Serde (JSON/TOML)
- **Logging**: Tracing
- **Testing**: Built-in Rust testing + integration tests

### Key Dependencies

- `eframe` - Cross-platform app framework
- `egui` - Immediate mode GUI library
- `tokio` - Async runtime
- `serde` - Serialization framework
- `anyhow` - Error handling
- `tracing` - Structured logging
- `uuid` - Unique identifiers

---

## 📚 Inspirations

- **[Zed](https://zed.dev/)**: Modern editor ergonomics and performance
- **[Obsidian](https://obsidian.md/)**: Plugin architecture and modularity  
- **[Scrivener](https://www.literatureandlatte.com/scrivener)**: Writing project organization
- **[FocusWriter](https://gottcode.org/focuswriter/)**: Distraction-free writing experience
- **[VS Code](https://code.visualstudio.com/)**: Extensible plugin ecosystem

---

## 🤝 Contributing

Cosmarium is in active development. Contributions are welcome!

### Ways to Contribute

- 🐛 **Bug Reports**: Open issues for bugs or unexpected behavior
- 💡 **Feature Requests**: Suggest new features or improvements  
- 🔧 **Code Contributions**: Submit pull requests
- 📝 **Documentation**: Improve docs and examples
- 🔌 **Plugin Development**: Create and share plugins

### Development Setup

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes and test: `cargo test`
4. Commit your changes: `git commit -m 'Add amazing feature'`
5. Push to the branch: `git push origin feature/amazing-feature`
6. Open a Pull Request

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🌟 Status

**Development Phase**: MVP Implementation (v0.1.0)

Cosmarium is currently in active development. The core architecture and basic functionality are implemented, with ongoing work on user experience and feature completeness.

**Current Focus**: Stabilizing the plugin API and expanding the built-in plugin ecosystem.
