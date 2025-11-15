//! # Layout management system for Cosmarium Core
//!
//! This module provides UI layout management functionality for the Cosmarium
//! creative writing software. It handles panel positioning, window management,
//! and workspace organization to create an optimal writing environment.
//!
//! The layout system supports dockable panels, floating windows, and
//! customizable workspace configurations that can be saved and restored.

use crate::{Error, Result, events::EventBus};
use cosmarium_plugin_api::{Panel, PanelPosition, Event, EventType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Layout management system for Cosmarium.
///
/// The [`LayoutManager`] handles all aspects of UI layout including panel
/// positioning, window management, and workspace configuration. It provides
/// APIs for creating, managing, and persisting layout configurations.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::layout::LayoutManager;
/// use cosmarium_core::events::EventBus;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// # tokio_test::block_on(async {
/// let event_bus = Arc::new(RwLock::new(EventBus::new()));
/// let mut manager = LayoutManager::new();
/// manager.initialize(event_bus).await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct LayoutManager {
    /// Current layout configuration
    current_layout: Layout,
    /// Saved layout configurations
    saved_layouts: HashMap<String, Layout>,
    /// Event bus for system communication
    event_bus: Option<Arc<RwLock<EventBus>>>,
    /// Whether the manager is initialized
    initialized: bool,
    /// Default layout configurations directory
    layouts_directory: PathBuf,
}

impl LayoutManager {
    /// Create a new layout manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::layout::LayoutManager;
    ///
    /// let manager = LayoutManager::new();
    /// ```
    pub fn new() -> Self {
        let layouts_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cosmarium")
            .join("layouts");

        Self {
            current_layout: Layout::default(),
            saved_layouts: HashMap::new(),
            event_bus: None,
            initialized: false,
            layouts_directory: layouts_dir,
        }
    }

    /// Initialize the layout manager.
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
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn initialize(&mut self, event_bus: Arc<RwLock<EventBus>>) -> Result<()> {
        if self.initialized {
            warn!("Layout manager is already initialized");
            return Ok(());
        }

        info!("Initializing layout manager");
        self.event_bus = Some(event_bus);

        // Ensure layouts directory exists
        if let Err(e) = tokio::fs::create_dir_all(&self.layouts_directory).await {
            warn!("Failed to create layouts directory: {}", e);
        }

        // Load saved layouts
        self.load_saved_layouts().await?;

        // Load default layout or create one
        if let Err(e) = self.load_layout("default").await {
            debug!("No default layout found, using built-in default: {}", e);
            self.current_layout = Layout::default();
        }

        self.initialized = true;
        info!("Layout manager initialized");
        Ok(())
    }

    /// Shutdown the layout manager.
    ///
    /// This method saves the current layout and cleans up resources.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.shutdown().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        info!("Shutting down layout manager");

        // Save current layout as default
        if let Err(e) = self.save_layout("default").await {
            error!("Failed to save default layout: {}", e);
        }

        self.initialized = false;
        info!("Layout manager shutdown completed");
        Ok(())
    }

    /// Get the current layout.
    ///
    /// # Returns
    ///
    /// Reference to the current layout configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let layout = manager.current_layout();
    /// assert_eq!(layout.name(), "default");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn current_layout(&self) -> &Layout {
        &self.current_layout
    }

    /// Get the current layout mutably.
    ///
    /// # Returns
    ///
    /// Mutable reference to the current layout configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let layout = manager.current_layout_mut();
    /// layout.set_name("custom");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn current_layout_mut(&mut self) -> &mut Layout {
        &mut self.current_layout
    }

    /// Add a panel to the current layout.
    ///
    /// # Arguments
    ///
    /// * `panel` - Panel to add to the layout
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use cosmarium_plugin_api::{Panel, PanelPosition, PanelSize};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    /// use uuid::Uuid;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let panel = Panel::new(
    ///     Uuid::new_v4(),
    ///     "Test Panel",
    ///     PanelPosition::Left,
    ///     PanelSize::Auto
    /// );
    /// manager.add_panel(panel);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn add_panel(&mut self, panel: Panel) {
        self.current_layout.add_panel(panel);
        self.emit_layout_changed();
    }

    /// Remove a panel from the current layout.
    ///
    /// # Arguments
    ///
    /// * `panel_id` - ID of the panel to remove
    ///
    /// # Returns
    ///
    /// True if the panel was removed, false if it wasn't found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    /// use uuid::Uuid;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let panel_id = Uuid::new_v4();
    /// let removed = manager.remove_panel(panel_id);
    /// assert!(!removed); // Panel doesn't exist
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn remove_panel(&mut self, panel_id: Uuid) -> bool {
        let removed = self.current_layout.remove_panel(panel_id);
        if removed {
            self.emit_layout_changed();
        }
        removed
    }

    /// Get a panel by ID.
    ///
    /// # Arguments
    ///
    /// * `panel_id` - ID of the panel to retrieve
    ///
    /// # Returns
    ///
    /// Reference to the panel if found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    /// use uuid::Uuid;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let panel_id = Uuid::new_v4();
    /// let panel = manager.get_panel(panel_id);
    /// assert!(panel.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn get_panel(&self, panel_id: Uuid) -> Option<&Panel> {
        self.current_layout.get_panel(panel_id)
    }

    /// Get a mutable panel by ID.
    ///
    /// # Arguments
    ///
    /// * `panel_id` - ID of the panel to retrieve
    ///
    /// # Returns
    ///
    /// Mutable reference to the panel if found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    /// use uuid::Uuid;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let panel_id = Uuid::new_v4();
    /// let panel = manager.get_panel_mut(panel_id);
    /// assert!(panel.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    /// Returns a mutable reference to a panel and emits layout changed event.
    /// # Example
    /// ```
    /// let mut layout = Layout::default();
    /// let panel_id = Uuid::new_v4();
    /// let panel = layout.get_panel_mut(panel_id);
    /// ```
    pub fn get_panel_mut(&mut self, panel_id: Uuid) -> Option<&mut Panel> {
        let found = self.current_layout.get_panel_mut(panel_id).is_some();
        if found {
            self.emit_layout_changed();
        }
        self.current_layout.get_panel_mut(panel_id)
    }

    /// Save the current layout with a name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name to save the layout under
    ///
    /// # Errors
    ///
    /// Returns an error if the layout cannot be saved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// manager.save_layout("my_layout").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn save_layout(&mut self, name: &str) -> Result<()> {
        let mut layout = self.current_layout.clone();
        layout.set_name(name);

        let layout_file = self.layouts_directory.join(format!("{}.json", name));
        let content = serde_json::to_string_pretty(&layout)
            .map_err(|e| Error::layout(format!("Failed to serialize layout: {}", e)))?;

        tokio::fs::write(&layout_file, content).await
            .map_err(|e| Error::layout(format!("Failed to write layout file: {}", e)))?;

        self.saved_layouts.insert(name.to_string(), layout);
        info!("Saved layout '{}'", name);
        Ok(())
    }

    /// Load a saved layout.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the layout to load
    ///
    /// # Errors
    ///
    /// Returns an error if the layout cannot be loaded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// // This will fail since no layout exists
    /// // let result = manager.load_layout("nonexistent").await;
    /// // assert!(result.is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn load_layout(&mut self, name: &str) -> Result<()> {
        let layout_file = self.layouts_directory.join(format!("{}.json", name));

        if !layout_file.exists() {
            return Err(Error::layout(format!("Layout '{}' not found", name)));
        }

        let content = tokio::fs::read_to_string(&layout_file).await
            .map_err(|e| Error::layout(format!("Failed to read layout file: {}", e)))?;

        let layout: Layout = serde_json::from_str(&content)
            .map_err(|e| Error::layout(format!("Failed to parse layout file: {}", e)))?;

        self.current_layout = layout;
        self.emit_layout_changed();

        info!("Loaded layout '{}'", name);
        Ok(())
    }

    /// List all saved layouts.
    ///
    /// # Returns
    ///
    /// Vector of layout names.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let layouts = manager.list_layouts();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn list_layouts(&self) -> Vec<String> {
        self.saved_layouts.keys().cloned().collect()
    }

    /// Update method called regularly for maintenance tasks.
    ///
    /// # Errors
    ///
    /// Returns an error if update operations fail.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{layout::LayoutManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = LayoutManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.update().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn update(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Layout-specific update logic would go here
        Ok(())
    }

    /// Load all saved layouts from disk.
    async fn load_saved_layouts(&mut self) -> Result<()> {
        if !self.layouts_directory.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&self.layouts_directory).await
            .map_err(|e| Error::layout(format!("Failed to read layouts directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::layout(format!("Failed to read directory entry: {}", e)))? {

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(layout) = serde_json::from_str::<Layout>(&content) {
                            self.saved_layouts.insert(name.to_string(), layout);
                            debug!("Loaded saved layout: {}", name);
                        }
                    }
                }
            }
        }

        info!("Loaded {} saved layouts", self.saved_layouts.len());
        Ok(())
    }

    /// Emit a layout changed event.
    fn emit_layout_changed(&self) {
        if let Some(ref event_bus) = self.event_bus {
            let event_bus = Arc::clone(event_bus);
            tokio::spawn(async move {
                let bus = event_bus.write().await;
                let event = Event::new(EventType::LayoutChanged, "Layout configuration changed");
                let _ = bus.emit(event).await;
            });
        }
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout configuration for the Cosmarium UI.
///
/// A layout defines the arrangement of panels, windows, and other UI elements
/// within the application workspace. Layouts can be saved and restored to
/// provide different working environments for different tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    /// Layout name
    name: String,
    /// Layout description
    description: String,
    /// Panels in this layout
    panels: HashMap<Uuid, Panel>,
    /// Window settings
    window_settings: WindowSettings,
    /// Custom properties
    properties: HashMap<String, serde_json::Value>,
}

impl Layout {
    /// Create a new layout.
    ///
    /// # Arguments
    ///
    /// * `name` - Layout name
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::layout::Layout;
    ///
    /// let layout = Layout::new("My Layout");
    /// assert_eq!(layout.name(), "My Layout");
    /// ```
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            panels: HashMap::new(),
            window_settings: WindowSettings::default(),
            properties: HashMap::new(),
        }
    }

    /// Get the layout name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the layout name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Get the layout description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Set the layout description.
    pub fn set_description(&mut self, description: &str) {
        self.description = description.to_string();
    }

    /// Add a panel to the layout.
    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.insert(panel.id, panel);
    }

    /// Remove a panel from the layout.
    pub fn remove_panel(&mut self, panel_id: Uuid) -> bool {
        self.panels.remove(&panel_id).is_some()
    }

    /// Get a panel by ID.
    pub fn get_panel(&self, panel_id: Uuid) -> Option<&Panel> {
        self.panels.get(&panel_id)
    }

    /// Get a mutable panel by ID.
    pub fn get_panel_mut(&mut self, panel_id: Uuid) -> Option<&mut Panel> {
        self.panels.get_mut(&panel_id)
    }

    /// Get all panels.
    pub fn panels(&self) -> impl Iterator<Item = &Panel> {
        self.panels.values()
    }

    /// Get all panels mutably.
    pub fn panels_mut(&mut self) -> impl Iterator<Item = &mut Panel> {
        self.panels.values_mut()
    }

    /// Get panels by position.
    pub fn panels_by_position(&self, position: PanelPosition) -> Vec<&Panel> {
        self.panels
            .values()
            .filter(|panel| panel.position == position)
            .collect()
    }

    /// Get window settings.
    pub fn window_settings(&self) -> &WindowSettings {
        &self.window_settings
    }

    /// Get mutable window settings.
    pub fn window_settings_mut(&mut self) -> &mut WindowSettings {
        &mut self.window_settings
    }

    /// Get a custom property.
    pub fn get_property<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.properties.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a custom property.
    pub fn set_property<T>(&mut self, key: &str, value: T) -> Result<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| Error::layout(format!("Failed to serialize property: {}", e)))?;
        self.properties.insert(key.to_string(), json_value);
        Ok(())
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new("default")
    }
}

/// Window-specific settings within a layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    /// Window width
    pub width: f32,
    /// Window height
    pub height: f32,
    /// Whether window is maximized
    pub maximized: bool,
    /// Window position X
    pub x: Option<f32>,
    /// Window position Y
    pub y: Option<f32>,
    /// Whether to always stay on top
    pub always_on_top: bool,
    /// Window decorations visible
    pub decorations: bool,
    /// Window transparency (0.0 = transparent, 1.0 = opaque)
    pub alpha: f32,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 800.0,
            maximized: false,
            x: None,
            y: None,
            always_on_top: false,
            decorations: true,
            alpha: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use cosmarium_plugin_api::{PanelPosition, PanelSize};

    #[tokio::test]
    async fn test_layout_manager_creation() {
        let manager = LayoutManager::new();
        assert!(!manager.initialized);
        assert_eq!(manager.current_layout.name(), "default");
    }

    #[tokio::test]
    async fn test_layout_manager_initialization() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = LayoutManager::new();

        assert!(manager.initialize(event_bus).await.is_ok());
        assert!(manager.initialized);
    }

    #[test]
    fn test_layout_creation() {
        let layout = Layout::new("Test Layout");
        assert_eq!(layout.name(), "Test Layout");
        assert!(layout.description().is_empty());
        assert_eq!(layout.panels().count(), 0);
    }

    #[test]
    fn test_layout_panel_management() {
        let mut layout = Layout::new("Test");
        let panel_id = Uuid::new_v4();
        let panel = Panel::new(panel_id, "Test Panel", PanelPosition::Left, PanelSize::Auto);

        layout.add_panel(panel);
        assert_eq!(layout.panels().count(), 1);
        assert!(layout.get_panel(panel_id).is_some());

        let removed = layout.remove_panel(panel_id);
        assert!(removed);
        assert_eq!(layout.panels().count(), 0);
    }

    #[test]
    fn test_layout_properties() {
        let mut layout = Layout::new("Test");

        layout.set_property("test_number", 42i32).unwrap();
        layout.set_property("test_string", "hello".to_string()).unwrap();

        assert_eq!(layout.get_property::<i32>("test_number"), Some(42));
        assert_eq!(layout.get_property::<String>("test_string"), Some("hello".to_string()));
        assert_eq!(layout.get_property::<i32>("nonexistent"), None);
    }

    #[test]
    fn test_window_settings_default() {
        let settings = WindowSettings::default();
        assert_eq!(settings.width, 1200.0);
        assert_eq!(settings.height, 800.0);
        assert!(!settings.maximized);
        assert!(settings.decorations);
        assert_eq!(settings.alpha, 1.0);
    }

    #[test]
    fn test_layout_panels_by_position() {
        let mut layout = Layout::new("Test");

        let panel1 = Panel::new(Uuid::new_v4(), "Left Panel", PanelPosition::Left, PanelSize::Auto);
        let panel2 = Panel::new(Uuid::new_v4(), "Right Panel", PanelPosition::Right, PanelSize::Auto);
        let panel3 = Panel::new(Uuid::new_v4(), "Another Left", PanelPosition::Left, PanelSize::Auto);

        layout.add_panel(panel1);
        layout.add_panel(panel2);
        layout.add_panel(panel3);

        let left_panels = layout.panels_by_position(PanelPosition::Left);
        let right_panels = layout.panels_by_position(PanelPosition::Right);

        assert_eq!(left_panels.len(), 2);
        assert_eq!(right_panels.len(), 1);
    }
}
