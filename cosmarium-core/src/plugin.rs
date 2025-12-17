//! # Plugin management system for Cosmarium Core
//!
//! This module provides the plugin management infrastructure for Cosmarium.
//! It handles plugin loading, unloading, lifecycle management, and inter-plugin
//! communication through a registry system.
//!
//! The plugin system is designed to be flexible and secure, supporting both
//! native Rust plugins and potentially other plugin types in the future.

use crate::{events::EventBus, Error, Result};
use cosmarium_plugin_api::{Plugin, PluginContext, PluginInfo, PluginType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Plugin management system for Cosmarium.
///
/// The [`PluginManager`] handles the entire plugin lifecycle including discovery,
/// loading, initialization, updates, and cleanup. It maintains a registry of
/// all available and loaded plugins and provides APIs for plugin management.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::PluginManager;
/// use cosmarium_core::events::EventBus;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// # tokio_test::block_on(async {
/// let event_bus = Arc::new(RwLock::new(EventBus::new()));
/// let mut manager = PluginManager::new();
/// manager.initialize(event_bus).await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct PluginManager {
    /// Registry of available plugins
    registry: PluginRegistry,
    /// Currently loaded plugins
    loaded_plugins: HashMap<String, Box<dyn Plugin>>,
    /// Plugin contexts for inter-plugin communication
    plugin_contexts: HashMap<String, PluginContext>,
    /// Event bus for system-wide communication
    event_bus: Option<Arc<RwLock<EventBus>>>,
    /// Plugin search directories
    plugin_directories: Vec<PathBuf>,
    /// Whether the plugin system is initialized
    initialized: bool,
}

impl PluginManager {
    /// Create a new plugin manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginManager;
    ///
    /// let manager = PluginManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            loaded_plugins: HashMap::new(),
            plugin_contexts: HashMap::new(),
            event_bus: None,
            plugin_directories: vec![
                PathBuf::from("plugins"),
                PathBuf::from("./target/debug"), // For development
            ],
            initialized: false,
        }
    }

    /// Initialize the plugin manager with an event bus.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - Shared event bus for inter-component communication
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{PluginManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = PluginManager::new();
    /// manager.initialize(event_bus).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn initialize(&mut self, event_bus: Arc<RwLock<EventBus>>) -> Result<()> {
        if self.initialized {
            warn!("Plugin manager is already initialized");
            return Ok(());
        }

        info!("Initializing plugin manager");
        self.event_bus = Some(event_bus);

        // Discover available plugins
        self.discover_plugins().await?;

        self.initialized = true;
        info!("Plugin manager initialized");
        Ok(())
    }

    /// Shutdown the plugin manager and all loaded plugins.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails for any plugin.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{PluginManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = PluginManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.shutdown().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        info!("Shutting down plugin manager");

        // Shutdown all loaded plugins
        for (name, plugin) in self.loaded_plugins.iter_mut() {
            if let Some(context) = self.plugin_contexts.get_mut(name) {
                if let Err(e) = plugin.shutdown(context).await {
                    error!("Failed to shutdown plugin '{}': {}", name, e);
                }
            }
        }

        self.loaded_plugins.clear();
        self.plugin_contexts.clear();
        self.initialized = false;

        info!("Plugin manager shutdown completed");
        Ok(())
    }

    /// Load a plugin by name.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - Name of the plugin to load
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin cannot be found or loaded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{PluginManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = PluginManager::new();
    /// manager.initialize(event_bus).await?;
    /// // manager.load_plugin("markdown-editor").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn load_plugin(&mut self, plugin_name: &str) -> Result<()> {
        if !self.initialized {
            return Err(Error::plugin("Plugin manager not initialized"));
        }

        if self.loaded_plugins.contains_key(plugin_name) {
            warn!("Plugin '{}' is already loaded", plugin_name);
            return Ok(());
        }

        info!("Loading plugin '{}'", plugin_name);

        // For now, we'll simulate plugin loading
        // In a real implementation, this would involve:
        // 1. Finding the plugin binary/library
        // 2. Loading it dynamically
        // 3. Instantiating the plugin

        // This is a placeholder - real implementation would use dynamic loading
        match plugin_name {
            "markdown-editor" => {
                // Create a mock plugin for testing
                let mut plugin = Box::new(MockPlugin::new(plugin_name));
                let mut context = PluginContext::new();

                // Initialize the plugin
                if let Err(e) = plugin.initialize(&mut context) {
                    return Err(Error::plugin(format!(
                        "Failed to initialize plugin '{}': {}",
                        plugin_name, e
                    )));
                }

                self.loaded_plugins.insert(plugin_name.to_string(), plugin);
                self.plugin_contexts
                    .insert(plugin_name.to_string(), context);

                info!("Plugin '{}' loaded successfully", plugin_name);
                Ok(())
            }
            _ => Err(Error::plugin(format!("Unknown plugin: {}", plugin_name))),
        }
    }

    /// Unload a plugin by name.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - Name of the plugin to unload
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin cannot be unloaded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{PluginManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = PluginManager::new();
    /// manager.initialize(event_bus).await?;
    /// // manager.load_plugin("test-plugin").await?;
    /// // manager.unload_plugin("test-plugin").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn unload_plugin(&mut self, plugin_name: &str) -> Result<()> {
        if !self.loaded_plugins.contains_key(plugin_name) {
            return Err(Error::plugin(format!(
                "Plugin '{}' is not loaded",
                plugin_name
            )));
        }

        info!("Unloading plugin '{}'", plugin_name);

        if let Some(mut plugin) = self.loaded_plugins.remove(plugin_name) {
            if let Some(mut context) = self.plugin_contexts.remove(plugin_name) {
                if let Err(e) = plugin.shutdown(&mut context).await {
                    error!("Error during plugin shutdown: {}", e);
                }
            }
        }

        info!("Plugin '{}' unloaded successfully", plugin_name);
        Ok(())
    }

    /// Get information about a loaded plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - Name of the plugin
    ///
    /// # Returns
    ///
    /// Plugin information if the plugin is loaded, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{PluginManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = PluginManager::new();
    /// manager.initialize(event_bus).await?;
    /// // let info = manager.get_plugin_info("markdown-editor");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn get_plugin_info(&self, plugin_name: &str) -> Option<PluginInfo> {
        self.loaded_plugins
            .get(plugin_name)
            .map(|plugin| plugin.info())
    }

    /// List all loaded plugins.
    ///
    /// # Returns
    ///
    /// Vector of plugin names that are currently loaded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginManager;
    ///
    /// let manager = PluginManager::new();
    /// let loaded = manager.list_loaded_plugins();
    /// ```
    pub fn list_loaded_plugins(&self) -> Vec<String> {
        self.loaded_plugins.keys().cloned().collect()
    }

    /// Check if a plugin is loaded.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - Name of the plugin to check
    ///
    /// # Returns
    ///
    /// True if the plugin is loaded, false otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginManager;
    ///
    /// let manager = PluginManager::new();
    /// assert!(!manager.is_plugin_loaded("test-plugin"));
    /// ```
    pub fn is_plugin_loaded(&self, plugin_name: &str) -> bool {
        self.loaded_plugins.contains_key(plugin_name)
    }

    /// Update all loaded plugins.
    ///
    /// This method calls the update method on all loaded plugins, allowing
    /// them to perform per-frame or periodic updates.
    ///
    /// # Errors
    ///
    /// Returns an error if any plugin update fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{PluginManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = PluginManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.update().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn update(&mut self) -> Result<()> {
        for (name, plugin) in self.loaded_plugins.iter_mut() {
            if let Some(context) = self.plugin_contexts.get_mut(name) {
                if let Err(e) = plugin.update(context) {
                    error!("Plugin '{}' update failed: {}", name, e);
                }
            }
        }
        Ok(())
    }

    /// Discover available plugins in the search directories.
    async fn discover_plugins(&mut self) -> Result<()> {
        debug!(
            "Discovering plugins in directories: {:?}",
            self.plugin_directories
        );

        let dirs: Vec<_> = self
            .plugin_directories
            .iter()
            .filter(|dir| dir.exists() && dir.is_dir())
            .cloned()
            .collect();

        for dir in dirs {
            self.scan_directory(&dir).await?;
        }

        info!(
            "Plugin discovery completed. Found {} plugins",
            self.registry.count()
        );
        Ok(())
    }

    /// Scan a directory for plugins.
    async fn scan_directory(&mut self, dir: &Path) -> Result<()> {
        debug!("Scanning directory: {:?}", dir);

        // In a real implementation, this would:
        // 1. Look for plugin manifest files (plugin.toml)
        // 2. Look for shared libraries (.so, .dll, .dylib)
        // 3. Validate plugin signatures if required
        // 4. Register discovered plugins

        // For now, this is a placeholder
        Ok(())
    }

    /// Get the plugin registry.
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for managing plugin metadata and discovery.
///
/// The [`PluginRegistry`] maintains information about available plugins,
/// their metadata, dependencies, and current status.
pub struct PluginRegistry {
    /// Available plugins
    plugins: HashMap<String, PluginInfo>,
    /// Plugin dependency graph
    dependencies: HashMap<String, Vec<String>>,
}

impl PluginRegistry {
    /// Create a new plugin registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginRegistry;
    ///
    /// let registry = PluginRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Register a plugin in the registry.
    ///
    /// # Arguments
    ///
    /// * `info` - Plugin information to register
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginRegistry;
    /// use cosmarium_plugin_api::PluginInfo;
    ///
    /// let mut registry = PluginRegistry::new();
    /// let info = PluginInfo::new("test", "1.0.0", "A test plugin", "Author");
    /// registry.register_plugin(info);
    /// ```
    pub fn register_plugin(&mut self, info: PluginInfo) {
        let name = info.name.clone();
        let deps = info.dependencies.clone();

        self.plugins.insert(name.clone(), info);
        self.dependencies.insert(name, deps);
    }

    /// Get plugin information by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name
    ///
    /// # Returns
    ///
    /// Plugin information if registered, None otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginRegistry;
    ///
    /// let registry = PluginRegistry::new();
    /// let info = registry.get_plugin("test");
    /// assert!(info.is_none());
    /// ```
    pub fn get_plugin(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// List all registered plugins.
    ///
    /// # Returns
    ///
    /// Vector of all plugin names in the registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginRegistry;
    ///
    /// let registry = PluginRegistry::new();
    /// let plugins = registry.list_plugins();
    /// assert!(plugins.is_empty());
    /// ```
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get the number of registered plugins.
    ///
    /// # Returns
    ///
    /// Number of plugins in the registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginRegistry;
    ///
    /// let registry = PluginRegistry::new();
    /// assert_eq!(registry.count(), 0);
    /// ```
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Check if a plugin is registered.
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name to check
    ///
    /// # Returns
    ///
    /// True if the plugin is registered, false otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::PluginRegistry;
    ///
    /// let registry = PluginRegistry::new();
    /// assert!(!registry.has_plugin("test"));
    /// ```
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock plugin implementation for testing purposes.
///
/// This is a simple plugin implementation used for testing and development
/// when actual plugins are not available.
pub struct MockPlugin {
    name: String,
    enabled: bool,
}

impl MockPlugin {
    /// Create a new mock plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - Plugin name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
        }
    }
}

use async_trait::async_trait;

#[async_trait]
impl Plugin for MockPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new(
            &self.name,
            "0.1.0",
            "Mock plugin for testing",
            "Cosmarium Core",
        )
    }

    fn initialize(&mut self, _ctx: &mut PluginContext) -> cosmarium_plugin_api::Result<()> {
        debug!("Mock plugin '{}' initialized", self.name);
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &mut PluginContext) -> cosmarium_plugin_api::Result<()> {
        debug!("Mock plugin '{}' shutdown", self.name);
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(
        &mut self,
        enabled: bool,
        _ctx: &mut PluginContext,
    ) -> cosmarium_plugin_api::Result<()> {
        self.enabled = enabled;
        Ok(())
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Utility
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(!manager.initialized);
        assert!(manager.loaded_plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_manager_initialization() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = PluginManager::new();

        assert!(manager.initialize(event_bus).await.is_ok());
        assert!(manager.initialized);
    }

    #[tokio::test]
    async fn test_plugin_loading() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = PluginManager::new();
        manager.initialize(event_bus).await.unwrap();

        assert!(manager.load_plugin("markdown-editor").await.is_ok());
        assert!(manager.is_plugin_loaded("markdown-editor"));
        assert_eq!(manager.list_loaded_plugins().len(), 1);
    }

    #[tokio::test]
    async fn test_plugin_unloading() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = PluginManager::new();
        manager.initialize(event_bus).await.unwrap();

        manager.load_plugin("markdown-editor").await.unwrap();
        assert!(manager.is_plugin_loaded("markdown-editor"));

        assert!(manager.unload_plugin("markdown-editor").await.is_ok());
        assert!(!manager.is_plugin_loaded("markdown-editor"));
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        assert_eq!(registry.count(), 0);

        let info = PluginInfo::new("test", "1.0.0", "Test plugin", "Author");
        registry.register_plugin(info);

        assert_eq!(registry.count(), 1);
        assert!(registry.has_plugin("test"));
        assert!(!registry.has_plugin("missing"));

        let retrieved = registry.get_plugin("test").unwrap();
        assert_eq!(retrieved.name, "test");
        assert_eq!(retrieved.version, "1.0.0");
    }

    #[test]
    fn test_mock_plugin() {
        let mut plugin = MockPlugin::new("test");
        let mut context = PluginContext::new();

        assert_eq!(plugin.info().name, "test");
        assert!(plugin.is_enabled());
        assert!(plugin.initialize(&mut context).is_ok());

        plugin.set_enabled(false, &mut context).unwrap();
        assert!(!plugin.is_enabled());
    }
}
