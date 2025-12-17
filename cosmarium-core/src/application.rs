//! # Cosmarium Core Application Module
//!
//! This module provides the main [`Application`] struct that serves as the central
//! coordinator for the Cosmarium creative writing software. It manages the plugin
//! system, configuration, and high-level application lifecycle.
//!
//! The application follows a modular architecture where all functionality is
//! provided through plugins, with the core handling coordination and infrastructure.

use crate::{
    config::Config, document::DocumentManager, error::Result, events::EventBus,
    layout::LayoutManager, plugin::PluginManager, project::ProjectManager,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Main application coordinator for Cosmarium.
///
/// The [`Application`] struct serves as the central hub that coordinates all
/// subsystems including plugins, projects, documents, and configuration.
/// It provides a unified interface for managing the application lifecycle.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::Application;
///
/// # tokio_test::block_on(async {
/// let mut app = Application::new();
/// app.initialize().await?;
///
/// // Load a plugin
/// let plugin_manager = app.plugin_manager();
/// let mut manager = plugin_manager.write().await;
/// manager.load_plugin("markdown-editor").await?;
///
/// // Application is ready to use
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct Application {
    /// Plugin management system
    plugin_manager: Arc<RwLock<PluginManager>>,
    /// Project management system
    project_manager: Arc<RwLock<ProjectManager>>,
    /// Document management system
    document_manager: Arc<RwLock<DocumentManager>>,
    /// UI layout management system
    layout_manager: Arc<RwLock<LayoutManager>>,
    /// Event bus for inter-component communication
    event_bus: Arc<RwLock<EventBus>>,
    /// Application configuration
    config: Arc<RwLock<Config>>,
    /// Whether the application has been initialized
    initialized: bool,
}

impl Application {
    /// Create a new application instance.
    ///
    /// This creates all the necessary subsystems but does not initialize them.
    /// Call [`initialize`](Self::initialize) to complete the setup process.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// assert!(!app.is_initialized());
    /// ```
    pub fn new() -> Self {
        info!("Creating new Cosmarium application instance");

        Self {
            plugin_manager: Arc::new(RwLock::new(PluginManager::new())),
            project_manager: Arc::new(RwLock::new(ProjectManager::new())),
            document_manager: Arc::new(RwLock::new(DocumentManager::new())),
            layout_manager: Arc::new(RwLock::new(LayoutManager::new())),
            event_bus: Arc::new(RwLock::new(EventBus::new())),
            config: Arc::new(RwLock::new(Config::default())),
            initialized: false,
        }
    }

    /// Initialize the application and all its subsystems.
    ///
    /// This method must be called before the application can be used.
    /// It loads configuration, initializes all managers, and sets up
    /// the inter-component communication channels.
    ///
    /// # Errors
    ///
    /// Returns an error if any subsystem fails to initialize.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// # tokio_test::block_on(async {
    /// let mut app = Application::new();
    /// app.initialize().await?;
    /// assert!(app.is_initialized());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            warn!("Application is already initialized");
            return Ok(());
        }

        info!("Initializing Cosmarium application");

        // Load configuration
        {
            let mut config = self.config.write().await;
            *config = Config::load_or_default()?;
            info!("Configuration loaded");
        }

        // Initialize event bus first as other systems depend on it
        {
            let mut event_bus = self.event_bus.write().await;
            event_bus.initialize().await?;
            info!("Event bus initialized");
        }

        // Initialize all managers
        {
            let mut plugin_manager = self.plugin_manager.write().await;
            plugin_manager
                .initialize(Arc::clone(&self.event_bus))
                .await?;
            info!("Plugin manager initialized");
        }

        {
            let mut project_manager = self.project_manager.write().await;
            project_manager
                .initialize(Arc::clone(&self.event_bus))
                .await?;
            info!("Project manager initialized");
        }

        {
            let mut document_manager = self.document_manager.write().await;
            document_manager
                .initialize(Arc::clone(&self.event_bus))
                .await?;
            info!("Document manager initialized");
        }

        {
            let mut layout_manager = self.layout_manager.write().await;
            layout_manager
                .initialize(Arc::clone(&self.event_bus))
                .await?;
            info!("Layout manager initialized");
        }

        self.initialized = true;
        info!("Application initialization completed successfully");
        Ok(())
    }

    /// Shutdown the application and all its subsystems.
    ///
    /// This method gracefully shuts down all managers, saves configuration,
    /// and cleans up resources. It should be called before the application exits.
    ///
    /// # Errors
    ///
    /// Returns an error if any subsystem fails to shutdown cleanly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// # tokio_test::block_on(async {
    /// let mut app = Application::new();
    /// app.initialize().await?;
    /// app.shutdown().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            warn!("Attempting to shutdown uninitialized application");
            return Ok(());
        }

        info!("Shutting down Cosmarium application");

        // Shutdown managers in reverse order
        {
            let mut layout_manager = self.layout_manager.write().await;
            if let Err(e) = layout_manager.shutdown().await {
                error!("Layout manager shutdown error: {}", e);
            }
        }

        {
            let mut document_manager = self.document_manager.write().await;
            if let Err(e) = document_manager.shutdown().await {
                error!("Document manager shutdown error: {}", e);
            }
        }

        {
            let mut project_manager = self.project_manager.write().await;
            if let Err(e) = project_manager.shutdown().await {
                error!("Project manager shutdown error: {}", e);
            }
        }

        {
            let mut plugin_manager = self.plugin_manager.write().await;
            if let Err(e) = plugin_manager.shutdown().await {
                error!("Plugin manager shutdown error: {}", e);
            }
        }

        // Save configuration
        {
            let config = self.config.read().await;
            if let Err(e) = config.save() {
                error!("Failed to save configuration: {}", e);
            }
        }

        // Shutdown event bus last
        {
            let mut event_bus = self.event_bus.write().await;
            if let Err(e) = event_bus.shutdown().await {
                error!("Event bus shutdown error: {}", e);
            }
        }

        self.initialized = false;
        info!("Application shutdown completed");
        Ok(())
    }

    /// Check if the application has been initialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// assert!(!app.is_initialized());
    /// ```
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get a reference to the plugin manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// let plugin_manager = app.plugin_manager();
    /// ```
    pub fn plugin_manager(&self) -> Arc<RwLock<PluginManager>> {
        Arc::clone(&self.plugin_manager)
    }

    /// Get a reference to the project manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// let project_manager = app.project_manager();
    /// ```
    pub fn project_manager(&self) -> Arc<RwLock<ProjectManager>> {
        Arc::clone(&self.project_manager)
    }

    /// Get a reference to the document manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// let document_manager = app.document_manager();
    /// ```
    pub fn document_manager(&self) -> Arc<RwLock<DocumentManager>> {
        Arc::clone(&self.document_manager)
    }

    /// Get a reference to the layout manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// let layout_manager = app.layout_manager();
    /// ```
    pub fn layout_manager(&self) -> Arc<RwLock<LayoutManager>> {
        Arc::clone(&self.layout_manager)
    }

    /// Get a reference to the event bus.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// let event_bus = app.event_bus();
    /// ```
    pub fn event_bus(&self) -> Arc<RwLock<EventBus>> {
        Arc::clone(&self.event_bus)
    }

    /// Get a reference to the configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// let app = Application::new();
    /// let config = app.config();
    /// ```
    pub fn config(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.config)
    }

    /// Run the application update cycle.
    ///
    /// This method should be called regularly (typically once per frame)
    /// to allow all subsystems to perform their update logic.
    ///
    /// # Errors
    ///
    /// Returns an error if any subsystem fails during the update cycle.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Application;
    ///
    /// # tokio_test::block_on(async {
    /// let mut app = Application::new();
    /// app.initialize().await?;
    ///
    /// // In your main loop
    /// app.update().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn update(&self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Update all managers
        {
            let mut plugin_manager = self.plugin_manager.write().await;
            plugin_manager.update().await?;
        }

        {
            let mut project_manager = self.project_manager.write().await;
            project_manager.update().await?;
        }

        {
            let mut document_manager = self.document_manager.write().await;
            document_manager.update().await?;
        }

        {
            let mut layout_manager = self.layout_manager.write().await;
            layout_manager.update().await?;
        }

        {
            let event_bus = self.event_bus.write().await;
            event_bus.process_events().await?;
        }

        Ok(())
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_application_creation() {
        let app = Application::new();
        assert!(!app.is_initialized());
    }

    #[tokio::test]
    async fn test_application_initialization() {
        let mut app = Application::new();
        assert!(app.initialize().await.is_ok());
        assert!(app.is_initialized());
    }

    #[tokio::test]
    async fn test_application_shutdown() {
        let mut app = Application::new();
        app.initialize().await.unwrap();
        assert!(app.shutdown().await.is_ok());
        assert!(!app.is_initialized());
    }

    #[tokio::test]
    async fn test_double_initialization() {
        let mut app = Application::new();
        app.initialize().await.unwrap();

        // Second initialization should not fail
        assert!(app.initialize().await.is_ok());
        assert!(app.is_initialized());
    }

    #[tokio::test]
    async fn test_update_cycle() {
        let mut app = Application::new();
        app.initialize().await.unwrap();

        // Update should work without errors
        assert!(app.update().await.is_ok());

        app.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_access() {
        let app = Application::new();

        // All managers should be accessible
        let _plugin_manager = app.plugin_manager();
        let _project_manager = app.project_manager();
        let _document_manager = app.document_manager();
        let _layout_manager = app.layout_manager();
        let _event_bus = app.event_bus();
        let _config = app.config();
    }
}
