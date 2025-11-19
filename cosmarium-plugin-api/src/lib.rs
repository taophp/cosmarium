//! # Cosmarium Plugin API
//!
//! This crate provides the API and traits that plugins must implement to integrate
//! with the Cosmarium creative writing software. It defines the core interfaces
//! for plugin lifecycle, UI rendering, and inter-plugin communication.
//!
//! ## Plugin Types
//!
//! Cosmarium supports several types of plugins:
//! - **Panel Plugins**: Provide UI panels (entities, notes, etc.)
//! - **Editor Plugins**: Extend text editing capabilities
//! - **Export Plugins**: Handle different export formats
//! - **AI Plugins**: Integrate AI functionality
//! - **Tool Plugins**: Provide utility functions
//!
//! ## Example Plugin
//!
//! ```rust
//! use cosmarium_plugin_api::{Plugin, PluginContext, PluginInfo, PanelPlugin};
//! use egui::Ui;
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn info(&self) -> PluginInfo {
//!         PluginInfo::new(
//!             "My Plugin",
//!             "0.1.0",
//!             "A sample plugin",
//!             "Plugin Author",
//!         )
//!     }
//!
//!     fn initialize(&mut self, ctx: &mut PluginContext) -> anyhow::Result<()> {
//!         // Plugin initialization
//!         Ok(())
//!     }
//! }
//!
//! impl PanelPlugin for MyPlugin {
//!     fn panel_title(&self) -> &str {
//!         "My Panel"
//!     }
//!
//!     fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
//!         ui.label("Hello from my plugin!");
//!     }
//! }
//! ```

pub mod context;
pub mod event;
pub mod panel;
pub mod plugin;


pub use context::{PluginContext, SharedState};
pub use event::{Event, EventHandler, EventType};
pub use panel::{Panel, PanelPlugin, PanelPosition, PanelSize, PanelContextMenuItem};
pub use plugin::{Plugin, PluginInfo, PluginType};


/// Result type used throughout the plugin API
pub type Result<T> = std::result::Result<T, anyhow::Error>;



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info_creation() {
        let info = PluginInfo::new("test", "1.0.0", "Test plugin", "Test Author");
        assert_eq!(info.name, "test");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.description, "Test plugin");
        assert_eq!(info.author, "Test Author");
        assert!(info.dependencies.is_empty());
        assert!(info.min_core_version.is_none());
    }

    #[test]
    fn test_plugin_info_with_dependency() {
        let info = PluginInfo::new("test", "1.0.0", "Test plugin", "Author")
            .with_dependency("markdown-editor");
        assert_eq!(info.dependencies, vec!["markdown-editor"]);
    }

    #[test]
    fn test_plugin_info_with_min_core_version() {
        let info = PluginInfo::new("test", "1.0.0", "Test plugin", "Author")
            .with_min_core_version("0.1.0");
        assert_eq!(info.min_core_version, Some("0.1.0".to_string()));
    }

    #[test]
    fn test_plugin_info_serialization() {
        let info = PluginInfo::new("test", "1.0.0", "Test plugin", "Author")
            .with_dependency("dep1")
            .with_min_core_version("0.1.0");

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: PluginInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.name, deserialized.name);
        assert_eq!(info.dependencies, deserialized.dependencies);
        assert_eq!(info.min_core_version, deserialized.min_core_version);
    }
}