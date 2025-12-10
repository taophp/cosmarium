//! Panel system for Cosmarium UI plugins.
//!
//! This module provides the traits and types for creating UI panels that can be
//! docked, resized, and managed by the Cosmarium layout system. Panel plugins
//! implement the [`PanelPlugin`] trait to provide custom UI components.
//!
//! # Example
//!
//! ```rust
//! use cosmarium_plugin_api::{PanelPlugin, Panel, PanelPosition, PanelSize, PluginContext};
//! use egui::Ui;
//!
//! struct MyPanel {
//!     content: String,
//! }
//!
//! impl PanelPlugin for MyPanel {
//!     fn panel_title(&self) -> &str {
//!         "My Custom Panel"
//!     }
//!
//!     fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
//!         ui.label("Hello from my panel!");
//!         ui.text_edit_singleline(&mut self.content);
//!     }
//!
//!     fn default_position(&self) -> PanelPosition {
//!         PanelPosition::Right
//!     }
//! }
//! ```

use crate::{PluginContext, Result};
use egui::Ui;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trait for plugins that provide UI panels.
///
/// Panel plugins create dockable UI components that can be positioned around
/// the main content area. They handle their own rendering and state management.
pub trait PanelPlugin: Send + Sync {
    /// Get the display title for this panel.
    ///
    /// This title is shown in the panel's tab and in the View menu.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelPlugin;
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct MyPanel;
    /// # impl PanelPlugin for MyPanel {
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn panel_title(&self) -> &str {
    ///     "Notes"
    /// }
    /// # }
    /// ```
    fn panel_title(&self) -> &str;

    /// Get the icon for this panel.
    ///
    /// This is used in the panel switcher (e.g. bottom tabs).
    /// Should return a single character string (emoji) or a short icon code.
    fn panel_icon(&self) -> &str {
        "🔌"
    }

    /// Update the panel state.
    ///
    /// This is called once per frame before rendering.
    fn update(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Render the panel's UI content.
    ///
    /// This method is called every frame when the panel is visible.
    /// Use the provided UI context to draw the panel's interface.
    ///
    /// # Arguments
    ///
    /// * `ui` - EGUI UI context for rendering
    /// * `ctx` - Plugin context for accessing shared state and services
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{PanelPlugin, PluginContext};
    /// use egui::Ui;
    ///
    /// # struct NotesPanel { notes: String }
    /// # impl PanelPlugin for NotesPanel {
    /// #     fn panel_title(&self) -> &str { "Notes" }
    ///
    /// fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
    ///     ui.heading("My Notes");
    ///     ui.separator();
    ///     
    ///     ui.text_edit_multiline(&mut self.notes);
    ///     
    ///     if ui.button("Save Notes").clicked() {
    ///         ctx.set_shared_state("notes_content", self.notes.clone());
    ///     }
    /// }
    /// # }
    /// ```
    fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext);

    /// Get the panel's unique identifier.
    ///
    /// If not overridden, this returns a UUID generated from the panel title.
    /// Override this method if you need a stable, predictable ID.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelPlugin;
    /// use uuid::Uuid;
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct MyPanel;
    /// # impl PanelPlugin for MyPanel {
    /// #     fn panel_title(&self) -> &str { "My Panel" }
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn panel_id(&self) -> Uuid {
    ///     Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    /// }
    /// # }
    /// ```
    fn panel_id(&self) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, self.panel_title().as_bytes())
    }

    /// Get the preferred position for this panel.
    ///
    /// This is used when the panel is first opened. The user can later
    /// move the panel to a different position.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{PanelPlugin, PanelPosition};
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct MyPanel;
    /// # impl PanelPlugin for MyPanel {
    /// #     fn panel_title(&self) -> &str { "My Panel" }
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn default_position(&self) -> PanelPosition {
    ///     PanelPosition::Left
    /// }
    /// # }
    /// ```
    fn default_position(&self) -> PanelPosition {
        PanelPosition::Right
    }

    /// Get the preferred size for this panel.
    ///
    /// This provides a hint to the layout system about the panel's
    /// preferred dimensions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{PanelPlugin, PanelSize};
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct MyPanel;
    /// # impl PanelPlugin for MyPanel {
    /// #     fn panel_title(&self) -> &str { "My Panel" }
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn default_size(&self) -> PanelSize {
    ///     PanelSize::Fixed { width: 300.0, height: 400.0 }
    /// }
    /// # }
    /// ```
    fn default_size(&self) -> PanelSize {
        PanelSize::Flexible {
            min_width: 200.0,
            min_height: 100.0,
            max_width: None,
            max_height: None,
        }
    }

    /// Check if the panel can be closed by the user.
    ///
    /// Some core panels might not be closable to ensure essential
    /// functionality remains available.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelPlugin;
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct CorePanel;
    /// # impl PanelPlugin for CorePanel {
    /// #     fn panel_title(&self) -> &str { "Core Panel" }
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn is_closable(&self) -> bool {
    ///     false // This panel cannot be closed
    /// }
    /// # }
    /// ```
    fn is_closable(&self) -> bool {
        true
    }

    /// Check if the panel should be shown by default when first opened.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelPlugin;
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct ImportantPanel;
    /// # impl PanelPlugin for ImportantPanel {
    /// #     fn panel_title(&self) -> &str { "Important Panel" }
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn default_open(&self) -> bool {
    ///     true // Show this panel by default
    /// }
    /// # }
    /// ```
    fn default_open(&self) -> bool {
        false
    }

    /// Called when the panel is opened.
    ///
    /// Use this method to initialize state or perform setup tasks
    /// when the panel becomes visible.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Plugin context for accessing shared state and services
    fn on_open(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Called when the panel is closed.
    ///
    /// Use this method to clean up resources or save state when
    /// the panel is hidden.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Plugin context for accessing shared state and services
    fn on_close(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }

    /// Get the panel's context menu items.
    ///
    /// Returns a list of menu items that should be shown when the user
    /// right-clicks on the panel's tab or header.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{PanelPlugin, PanelContextMenuItem};
    /// # use cosmarium_plugin_api::PluginContext;
    /// # use egui::Ui;
    /// # struct MyPanel;
    /// # impl PanelPlugin for MyPanel {
    /// #     fn panel_title(&self) -> &str { "My Panel" }
    /// #     fn render_panel(&mut self, _: &mut Ui, _: &mut PluginContext) {}
    ///
    /// fn context_menu_items(&self) -> Vec<PanelContextMenuItem> {
    ///     vec![
    ///         PanelContextMenuItem::new("refresh", "Refresh Data"),
    ///         PanelContextMenuItem::separator(),
    ///         PanelContextMenuItem::new("settings", "Panel Settings"),
    ///     ]
    /// }
    /// # }
    /// ```
    fn context_menu_items(&self) -> Vec<PanelContextMenuItem> {
        Vec::new()
    }

    /// Handle a context menu item being selected.
    ///
    /// Called when the user clicks on one of the items returned by
    /// [`context_menu_items`].
    ///
    /// # Arguments
    ///
    /// * `item_id` - ID of the selected menu item
    /// * `ctx` - Plugin context for accessing shared state and services
    fn handle_context_menu(&mut self, item_id: &str, ctx: &mut PluginContext) -> Result<()> {
        let _ = (item_id, ctx);
        Ok(())
    }
}

/// Represents a panel instance with its state and configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    /// Unique identifier for this panel
    pub id: Uuid,
    /// Display title of the panel
    pub title: String,
    /// Current position in the layout
    pub position: PanelPosition,
    /// Current size configuration
    pub size: PanelSize,
    /// Whether the panel is currently visible
    pub visible: bool,
    /// Whether the panel can be closed
    pub closable: bool,
}

impl Panel {
    /// Create a new panel instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Panel, PanelPosition, PanelSize};
    /// use uuid::Uuid;
    ///
    /// let panel = Panel::new(
    ///     Uuid::new_v4(),
    ///     "My Panel",
    ///     PanelPosition::Left,
    ///     PanelSize::Flexible {
    ///         min_width: 200.0,
    ///         min_height: 100.0,
    ///         max_width: Some(500.0),
    ///         max_height: None,
    ///     }
    /// );
    /// ```
    pub fn new<S: Into<String>>(
        id: Uuid,
        title: S,
        position: PanelPosition,
        size: PanelSize,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            position,
            size,
            visible: false,
            closable: true,
        }
    }

    /// Set the panel's visibility.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Panel, PanelPosition, PanelSize};
    /// use uuid::Uuid;
    ///
    /// let mut panel = Panel::new(Uuid::new_v4(), "Test", PanelPosition::Left, PanelSize::Auto);
    /// panel.set_visible(true);
    /// assert!(panel.visible);
    /// ```
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Set whether the panel can be closed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Panel, PanelPosition, PanelSize};
    /// use uuid::Uuid;
    ///
    /// let mut panel = Panel::new(Uuid::new_v4(), "Core", PanelPosition::Left, PanelSize::Auto);
    /// panel.set_closable(false);
    /// assert!(!panel.closable);
    /// ```
    pub fn set_closable(&mut self, closable: bool) {
        self.closable = closable;
    }
}

/// Position where a panel can be docked in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelPosition {
    /// Docked to the left side of the main content
    Left,
    /// Docked to the right side of the main content
    Right,
    /// Docked to the top of the main content
    Top,
    /// Docked to the bottom of the main content
    Bottom,
    /// Floating as a separate window
    Floating,
    /// Centered in the main content area (typically for modal panels)
    Center,
}

impl PanelPosition {
    /// Get a human-readable name for the position.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelPosition;
    ///
    /// assert_eq!(PanelPosition::Left.display_name(), "Left");
    /// assert_eq!(PanelPosition::Floating.display_name(), "Floating");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            PanelPosition::Left => "Left",
            PanelPosition::Right => "Right",
            PanelPosition::Top => "Top",
            PanelPosition::Bottom => "Bottom",
            PanelPosition::Floating => "Floating",
            PanelPosition::Center => "Center",
        }
    }

    /// Get all available positions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelPosition;
    ///
    /// let positions = PanelPosition::all();
    /// assert!(positions.contains(&PanelPosition::Left));
    /// assert!(positions.contains(&PanelPosition::Floating));
    /// ```
    pub fn all() -> Vec<PanelPosition> {
        vec![
            PanelPosition::Left,
            PanelPosition::Right,
            PanelPosition::Top,
            PanelPosition::Bottom,
            PanelPosition::Floating,
            PanelPosition::Center,
        ]
    }
}

/// Size configuration for a panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PanelSize {
    /// Automatically sized based on content
    Auto,
    /// Fixed size in pixels
    Fixed { width: f32, height: f32 },
    /// Flexible size with constraints
    Flexible {
        min_width: f32,
        min_height: f32,
        max_width: Option<f32>,
        max_height: Option<f32>,
    },
    /// Percentage of available space
    Percentage { width_pct: f32, height_pct: f32 },
}

impl PanelSize {
    /// Create a new fixed size.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelSize;
    ///
    /// let size = PanelSize::fixed(300.0, 400.0);
    /// ```
    pub fn fixed(width: f32, height: f32) -> Self {
        Self::Fixed { width, height }
    }

    /// Create a new flexible size with minimum constraints.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelSize;
    ///
    /// let size = PanelSize::flexible(200.0, 100.0);
    /// ```
    pub fn flexible(min_width: f32, min_height: f32) -> Self {
        Self::Flexible {
            min_width,
            min_height,
            max_width: None,
            max_height: None,
        }
    }

    /// Create a new percentage-based size.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelSize;
    ///
    /// let size = PanelSize::percentage(0.3, 0.5); // 30% width, 50% height
    /// ```
    pub fn percentage(width_pct: f32, height_pct: f32) -> Self {
        Self::Percentage {
            width_pct,
            height_pct,
        }
    }
}

impl Default for PanelSize {
    fn default() -> Self {
        Self::Auto
    }
}

/// Context menu item for panels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelContextMenuItem {
    /// Unique identifier for the menu item
    pub id: String,
    /// Display text for the menu item
    pub label: String,
    /// Whether this is a separator item
    pub is_separator: bool,
    /// Whether the item is enabled
    pub enabled: bool,
}

impl PanelContextMenuItem {
    /// Create a new menu item.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelContextMenuItem;
    ///
    /// let item = PanelContextMenuItem::new("save", "Save Panel State");
    /// ```
    pub fn new<S: Into<String>>(id: S, label: S) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            is_separator: false,
            enabled: true,
        }
    }

    /// Create a separator menu item.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelContextMenuItem;
    ///
    /// let separator = PanelContextMenuItem::separator();
    /// assert!(separator.is_separator);
    /// ```
    pub fn separator() -> Self {
        Self {
            id: "separator".to_string(),
            label: String::new(),
            is_separator: true,
            enabled: false,
        }
    }

    /// Set whether the menu item is enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PanelContextMenuItem;
    ///
    /// let item = PanelContextMenuItem::new("delete", "Delete Item")
    ///     .with_enabled(false);
    /// assert!(!item.enabled);
    /// ```
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginContext;

    struct TestPanel {
        title: String,
        content: String,
    }

    impl PanelPlugin for TestPanel {
        fn panel_title(&self) -> &str {
            &self.title
        }

        fn render_panel(&mut self, ui: &mut Ui, _ctx: &mut PluginContext) {
            ui.label(&self.content);
        }

        fn default_position(&self) -> PanelPosition {
            PanelPosition::Left
        }

        fn default_size(&self) -> PanelSize {
            PanelSize::fixed(300.0, 400.0)
        }

        fn context_menu_items(&self) -> Vec<PanelContextMenuItem> {
            vec![
                PanelContextMenuItem::new("clear", "Clear Content"),
                PanelContextMenuItem::separator(),
                PanelContextMenuItem::new("settings", "Settings"),
            ]
        }
    }

    #[test]
    fn test_panel_creation() {
        let id = Uuid::new_v4();
        let panel = Panel::new(id, "Test Panel", PanelPosition::Right, PanelSize::Auto);

        assert_eq!(panel.id, id);
        assert_eq!(panel.title, "Test Panel");
        assert_eq!(panel.position, PanelPosition::Right);
        assert_eq!(panel.size, PanelSize::Auto);
        assert!(!panel.visible);
        assert!(panel.closable);
    }

    #[test]
    fn test_panel_visibility() {
        let mut panel = Panel::new(
            Uuid::new_v4(),
            "Test",
            PanelPosition::Left,
            PanelSize::Auto,
        );

        assert!(!panel.visible);
        panel.set_visible(true);
        assert!(panel.visible);
        panel.set_visible(false);
        assert!(!panel.visible);
    }

    #[test]
    fn test_panel_position_display_name() {
        assert_eq!(PanelPosition::Left.display_name(), "Left");
        assert_eq!(PanelPosition::Right.display_name(), "Right");
        assert_eq!(PanelPosition::Top.display_name(), "Top");
        assert_eq!(PanelPosition::Bottom.display_name(), "Bottom");
        assert_eq!(PanelPosition::Floating.display_name(), "Floating");
        assert_eq!(PanelPosition::Center.display_name(), "Center");
    }

    #[test]
    fn test_panel_size_constructors() {
        let fixed = PanelSize::fixed(100.0, 200.0);
        assert_eq!(fixed, PanelSize::Fixed { width: 100.0, height: 200.0 });

        let flexible = PanelSize::flexible(50.0, 75.0);
        assert_eq!(flexible, PanelSize::Flexible {
            min_width: 50.0,
            min_height: 75.0,
            max_width: None,
            max_height: None,
        });

        let percentage = PanelSize::percentage(0.5, 0.3);
        assert_eq!(percentage, PanelSize::Percentage {
            width_pct: 0.5,
            height_pct: 0.3,
        });
    }

    #[test]
    fn test_context_menu_item() {
        let item = PanelContextMenuItem::new("test", "Test Item");
        assert_eq!(item.id, "test");
        assert_eq!(item.label, "Test Item");
        assert!(!item.is_separator);
        assert!(item.enabled);

        let separator = PanelContextMenuItem::separator();
        assert!(separator.is_separator);
        assert!(!separator.enabled);

        let disabled_item = PanelContextMenuItem::new("disabled", "Disabled Item")
            .with_enabled(false);
        assert!(!disabled_item.enabled);
    }

    #[test]
    fn test_panel_plugin_defaults() {
        let panel = TestPanel {
            title: "Test Panel".to_string(),
            content: "Hello, World!".to_string(),
        };

        assert_eq!(panel.panel_title(), "Test Panel");
        assert_eq!(panel.default_position(), PanelPosition::Left);
        assert!(panel.is_closable());
        assert!(!panel.default_open());
        
        let context_items = panel.context_menu_items();
        assert_eq!(context_items.len(), 3);
        assert_eq!(context_items[0].id, "clear");
        assert!(context_items[1].is_separator);
        assert_eq!(context_items[2].id, "settings");
    }

    #[test]
    fn test_panel_id_generation() {
        let panel1 = TestPanel {
            title: "Same Title".to_string(),
            content: "Content 1".to_string(),
        };
        let panel2 = TestPanel {
            title: "Same Title".to_string(),
            content: "Content 2".to_string(),
        };

        // Panels with the same title should have the same ID
        assert_eq!(panel1.panel_id(), panel2.panel_id());

        let panel3 = TestPanel {
            title: "Different Title".to_string(),
            content: "Content 3".to_string(),
        };

        // Panels with different titles should have different IDs
        assert_ne!(panel1.panel_id(), panel3.panel_id());
    }
}