//! # Cosmarium Core
//!
//! Core functionality for the Cosmarium creative writing software.
//! This crate provides the foundational systems for project management,
//! plugin architecture, and application state management.
//!
//! ## Architecture
//!
//! Cosmarium follows a modular plugin-based architecture where:
//! - The core provides basic infrastructure and plugin management
//! - All features are implemented as plugins
//! - Plugins can communicate through events and shared state
//!
//! ## Example
//!
//! ```rust
//! use cosmarium_core::{Application, PluginManager};
//!
//! # tokio_test::block_on(async {
//! let mut app = Application::new();
//! app.initialize().await?;
//! let plugin_manager = app.plugin_manager();
//! let mut manager = plugin_manager.write().await;
//! manager.load_plugin("markdown-editor").await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # });
//! ```

pub mod application;
pub mod config;
pub mod document;
pub mod error;
pub mod events;
pub mod git;
pub mod layout;
pub mod plugin;
pub mod project;
pub mod session;


pub use application::Application;
pub use config::Config;
pub use document::{Document, DocumentManager};
pub use error::{Error, Result};
pub use cosmarium_plugin_api::event::{Event, EventType};
pub use events::EventBus;
pub use layout::{Layout, LayoutManager};
pub use plugin::{PluginManager, PluginRegistry};
pub use project::{Project, ProjectManager};
pub use session::Session;

/// Initialize tracing for the application
///
/// This sets up structured logging for the entire application.
///
/// # Example
///
/// ```rust
/// cosmarium_core::init_tracing();
/// tracing::info!("Application started");
/// ```
pub fn init_tracing() {
    // Try to initialize a tracing subscriber but avoid panicking if a global
    // subscriber has already been installed by another logger (for example
    // env_logger). Use `try_init()` to attempt installation and ignore the
    // error when the global subscriber is already set.
    //
    // This keeps startup robust when multiple logging initializers are used,
    // while still ensuring tracing events are captured when possible.
    let _ = tracing_subscriber::fmt::try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing() {
        // Should not panic
        init_tracing();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_application_lifecycle() {
        let mut app = Application::new();
        assert!(app.initialize().await.is_ok());
        assert!(app.is_initialized());
        assert!(app.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_core_plugin_loading() {
        let mut app = Application::new();
        app.initialize().await.unwrap();
        
        let plugin_manager_lock = app.plugin_manager();
        let mut plugin_manager = plugin_manager_lock.write().await;
        assert!(plugin_manager.load_plugin("markdown-editor").await.is_ok());
        assert!(plugin_manager.is_plugin_loaded("markdown-editor"));
    }
}