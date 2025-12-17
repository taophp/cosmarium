//! Core plugin trait and types for the Cosmarium plugin system.
//!
//! This module defines the main [`Plugin`] trait that all plugins must implement,
//! along with supporting types for plugin categorization and lifecycle management.

use crate::{PluginContext, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Plugin metadata information
///
/// Contains basic information about a plugin that is displayed
/// in the plugin manager and used for dependency resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name (must be unique)
    pub name: String,
    /// Plugin version (semantic versioning)
    pub version: String,
    /// Brief description of the plugin's functionality
    pub description: String,
    /// Plugin author(s)
    pub author: String,
    /// Plugin dependencies (other plugin names)
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Minimum Cosmarium core version required
    #[serde(default)]
    pub min_core_version: Option<String>,
}

impl PluginInfo {
    /// Create a new PluginInfo with minimal required fields
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginInfo;
    ///
    /// let info = PluginInfo::new(
    ///     "my-plugin",
    ///     "1.0.0",
    ///     "A sample plugin",
    ///     "Plugin Author"
    /// );
    /// ```
    pub fn new(name: &str, version: &str, description: &str, author: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            author: author.to_string(),
            dependencies: Vec::new(),
            min_core_version: None,
        }
    }

    /// Add a dependency to this plugin
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginInfo;
    ///
    /// let mut info = PluginInfo::new("my-plugin", "1.0.0", "A plugin", "Author");
    /// info.with_dependency("markdown-editor");
    /// ```
    pub fn with_dependency<S: Into<String>>(mut self, dependency: S) -> Self {
        self.dependencies.push(dependency.into());
        self
    }

    /// Set the minimum required core version
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginInfo;
    ///
    /// let info = PluginInfo::new("my-plugin", "1.0.0", "A plugin", "Author")
    ///     .with_min_core_version("0.1.0");
    /// ```
    pub fn with_min_core_version<S: Into<String>>(mut self, version: S) -> Self {
        self.min_core_version = Some(version.into());
        self
    }
}

/// The main trait that all Cosmarium plugins must implement.
///
/// This trait defines the core lifecycle methods and metadata for plugins.
/// Plugins should also implement one or more specialized traits like
/// [`PanelPlugin`](crate::PanelPlugin), [`EditorPlugin`](crate::EditorPlugin), etc.
///
/// # Example
///
/// ```rust
/// use cosmarium_plugin_api::{Plugin, PluginInfo, PluginContext};
///
/// struct MyPlugin {
///     enabled: bool,
/// }
///
/// impl Plugin for MyPlugin {
///     fn info(&self) -> PluginInfo {
///         PluginInfo::new(
///             "my-plugin",
///             "1.0.0",
///             "A simple example plugin",
///             "Plugin Developer"
///         )
///     }
///
///     fn initialize(&mut self, ctx: &mut PluginContext) -> anyhow::Result<()> {
///         self.enabled = true;
///         Ok(())
///     }
///
///     fn is_enabled(&self) -> bool {
///         self.enabled
///     }
/// }
/// ```
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Returns metadata information about this plugin.
    ///
    /// This information is used by the plugin manager for display,
    /// dependency resolution, and version compatibility checks.
    fn info(&self) -> PluginInfo;

    /// Initialize the plugin.
    ///
    /// Called once when the plugin is first loaded. Use this method
    /// to set up any required state, register event handlers, or
    /// perform other initialization tasks.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Plugin context providing access to shared state and services
    ///
    /// # Errors
    ///
    /// Return an error if initialization fails. The plugin will be marked
    /// as failed and will not be activated.
    fn initialize(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Shutdown the plugin.
    ///
    /// Called when the plugin is being unloaded or the application is shutting down.
    /// Use this method to clean up resources, save state, or perform other cleanup tasks.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Plugin context providing access to shared state and services
    async fn shutdown(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Check if the plugin is currently enabled.
    ///
    /// Disabled plugins are loaded but their functionality is not active.
    /// This allows users to temporarily disable plugins without unloading them.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::Plugin;
    /// # use cosmarium_plugin_api::{PluginInfo, PluginContext};
    /// # struct MyPlugin { enabled: bool }
    /// # impl Plugin for MyPlugin {
    /// #     fn info(&self) -> PluginInfo { PluginInfo::new("test", "1.0.0", "test", "test") }
    /// #     fn initialize(&mut self, _: &mut PluginContext) -> anyhow::Result<()> { Ok(()) }
    ///
    /// fn is_enabled(&self) -> bool {
    ///     self.enabled
    /// }
    /// # }
    /// ```
    fn is_enabled(&self) -> bool {
        true
    }

    /// Enable or disable the plugin.
    ///
    /// Called when the user changes the plugin's enabled state through the UI.
    /// Plugins can override this to perform specific actions when being enabled/disabled.
    ///
    /// # Arguments
    ///
    /// * `enabled` - New enabled state
    /// * `ctx` - Plugin context providing access to shared state and services
    fn set_enabled(&mut self, enabled: bool, ctx: &mut PluginContext) -> Result<()> {
        let _ = (enabled, ctx);
        Ok(())
    }

    /// Get the plugin's type category.
    ///
    /// Used for organizing plugins in the UI and determining which
    /// specialized traits the plugin implements.
    fn plugin_type(&self) -> PluginType {
        PluginType::Utility
    }

    /// Called when the plugin should update its state.
    ///
    /// This is called regularly (typically once per frame) and allows
    /// plugins to perform background tasks, update state, or handle
    /// time-based operations.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Plugin context providing access to shared state and services
    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Categories of plugins supported by Cosmarium.
///
/// These categories help organize plugins in the UI and determine
/// which specialized traits a plugin implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginType {
    /// Panel plugins that provide UI panels (entities, notes, etc.)
    Panel,
    /// Editor plugins that extend text editing capabilities
    Editor,
    /// Export plugins that handle different output formats
    Export,
    /// AI plugins that provide artificial intelligence features
    AI,
    /// Analysis plugins that provide text or project analysis
    Analysis,
    /// Import plugins that handle reading external formats
    Import,
    /// Collaboration plugins for multi-user features
    Collaboration,
    /// Theme plugins that modify the application appearance
    Theme,
    /// Utility plugins that provide miscellaneous tools
    Utility,
}

impl PluginType {
    /// Get a human-readable name for the plugin type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginType;
    ///
    /// assert_eq!(PluginType::Panel.display_name(), "Panel");
    /// assert_eq!(PluginType::AI.display_name(), "AI Assistant");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            PluginType::Panel => "Panel",
            PluginType::Editor => "Editor",
            PluginType::Export => "Export",
            PluginType::AI => "AI Assistant",
            PluginType::Analysis => "Analysis",
            PluginType::Import => "Import",
            PluginType::Collaboration => "Collaboration",
            PluginType::Theme => "Theme",
            PluginType::Utility => "Utility",
        }
    }

    /// Get all available plugin types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginType;
    ///
    /// let types = PluginType::all();
    /// assert!(types.contains(&PluginType::Panel));
    /// assert!(types.contains(&PluginType::AI));
    /// ```
    pub fn all() -> Vec<PluginType> {
        vec![
            PluginType::Panel,
            PluginType::Editor,
            PluginType::Export,
            PluginType::AI,
            PluginType::Analysis,
            PluginType::Import,
            PluginType::Collaboration,
            PluginType::Theme,
            PluginType::Utility,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginContext;

    struct TestPlugin {
        enabled: bool,
    }

    impl Plugin for TestPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo::new("test-plugin", "1.0.0", "A test plugin", "Test Author")
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn set_enabled(&mut self, enabled: bool, _ctx: &mut PluginContext) -> Result<()> {
            self.enabled = enabled;
            Ok(())
        }

        fn plugin_type(&self) -> PluginType {
            PluginType::Panel
        }
    }

    #[test]
    fn test_plugin_type_display_name() {
        assert_eq!(PluginType::Panel.display_name(), "Panel");
        assert_eq!(PluginType::AI.display_name(), "AI Assistant");
        assert_eq!(PluginType::Utility.display_name(), "Utility");
    }

    #[test]
    fn test_plugin_type_all() {
        let types = PluginType::all();
        assert_eq!(types.len(), 9);
        assert!(types.contains(&PluginType::Panel));
        assert!(types.contains(&PluginType::AI));
        assert!(types.contains(&PluginType::Utility));
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = TestPlugin { enabled: false };
        let mut ctx = PluginContext::new(); // This will need to be implemented

        // Test initialization
        assert!(plugin.initialize(&mut ctx).is_ok());

        // Test enabled state
        assert!(!plugin.is_enabled());
        assert!(plugin.set_enabled(true, &mut ctx).is_ok());
        assert!(plugin.is_enabled());

        // Test plugin info
        let info = plugin.info();
        assert_eq!(info.name, "test-plugin");
        assert_eq!(info.version, "1.0.0");

        // Test plugin type
        assert_eq!(plugin.plugin_type(), PluginType::Panel);

        // Test update
        assert!(plugin.update(&mut ctx).is_ok());

        // Test shutdown
        assert!(plugin.shutdown(&mut ctx).await.is_ok());
    }
}
