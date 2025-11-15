//! Event system for inter-plugin communication in Cosmarium.
//!
//! This module provides a type-safe event system that allows plugins to communicate
//! with each other and with the core application without tight coupling. Events are
//! dispatched through the [`PluginContext`] and handled by registered [`EventHandler`]s.
//!
//! # Example
//!
//! ```rust
//! use cosmarium_plugin_api::{Event, EventHandler, EventType};
//!
//! struct MyEventHandler;
//!
//! impl EventHandler for MyEventHandler {
//!     fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
//!         match event.event_type() {
//!             EventType::DocumentChanged => {
//!                 println!("Document was changed: {}", event.data());
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core trait for handling events in the plugin system.
///
/// Event handlers are registered with the [`PluginContext`] and are called
/// when matching events are emitted by plugins or the core application.
pub trait EventHandler: Send + Sync {
    /// Handle an incoming event.
    ///
    /// This method is called whenever an event of the registered type is emitted.
    /// Implementations should be efficient as they may be called frequently.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Errors
    ///
    /// Return an error if event handling fails. The error will be logged but
    /// will not prevent other handlers from being called.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventHandler, EventType};
    ///
    /// struct DocumentHandler;
    ///
    /// impl EventHandler for DocumentHandler {
    ///     fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
    ///         if let EventType::DocumentChanged = event.event_type() {
    ///             println!("Document updated: {}", event.data());
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn handle(&mut self, event: &Event) -> anyhow::Result<()>;
}

/// An event that can be emitted and handled by the plugin system.
///
/// Events carry information about something that happened in the application,
/// such as a document being changed, a plugin being loaded, or user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique identifier for this event instance
    id: Uuid,
    /// Type of the event
    event_type: EventType,
    /// Optional data payload
    data: String,
    /// Additional metadata
    metadata: HashMap<String, String>,
    /// Timestamp when the event was created
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl Event {
    /// Create a new event with the specified type and data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let event = Event::new(
    ///     EventType::DocumentChanged,
    ///     "Document content updated".to_string()
    /// );
    /// ```
    pub fn new<S: Into<String>>(event_type: EventType, data: S) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            data: data.into(),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new event with additional metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    /// use std::collections::HashMap;
    ///
    /// let mut metadata = HashMap::new();
    /// metadata.insert("plugin".to_string(), "markdown-editor".to_string());
    ///
    /// let event = Event::with_metadata(
    ///     EventType::PluginLoaded,
    ///     "Plugin loaded successfully",
    ///     metadata
    /// );
    /// ```
    pub fn with_metadata<S: Into<String>>(
        event_type: EventType,
        data: S,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            data: data.into(),
            metadata,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get the unique identifier of this event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let event = Event::new(EventType::DocumentChanged, "data");
    /// let id = event.id();
    /// ```
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the type of this event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let event = Event::new(EventType::DocumentChanged, "data");
    /// assert_eq!(event.event_type(), EventType::DocumentChanged);
    /// ```
    pub fn event_type(&self) -> EventType {
        self.event_type
    }

    /// Get the data payload of this event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let event = Event::new(EventType::DocumentChanged, "Hello, World!");
    /// assert_eq!(event.data(), "Hello, World!");
    /// ```
    pub fn data(&self) -> &str {
        &self.data
    }

    /// Get the metadata associated with this event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    /// use std::collections::HashMap;
    ///
    /// let mut metadata = HashMap::new();
    /// metadata.insert("key".to_string(), "value".to_string());
    ///
    /// let event = Event::with_metadata(EventType::Custom, "data", metadata);
    /// assert_eq!(event.metadata().get("key"), Some(&"value".to_string()));
    /// ```
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Get the timestamp when this event was created.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let event = Event::new(EventType::DocumentChanged, "data");
    /// let timestamp = event.timestamp();
    /// ```
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    /// Add or update a metadata entry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let mut event = Event::new(EventType::DocumentChanged, "data");
    /// event.set_metadata("plugin", "my-plugin");
    /// assert_eq!(event.metadata().get("plugin"), Some(&"my-plugin".to_string()));
    /// ```
    pub fn set_metadata<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get a specific metadata value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// let mut event = Event::new(EventType::DocumentChanged, "data");
    /// event.set_metadata("level", "info");
    /// assert_eq!(event.get_metadata("level"), Some("info"));
    /// ```
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

/// Types of events that can be emitted in the Cosmarium plugin system.
///
/// This enum defines all the standard event types that plugins and the core
/// application can emit and handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    // Document events
    /// Document content was changed
    DocumentChanged,
    /// A new document was created
    DocumentCreated,
    /// A document was opened
    DocumentOpened,
    /// A document was saved
    DocumentSaved,
    /// A document was closed
    DocumentClosed,

    // Project events
    /// A project was created
    ProjectCreated,
    /// A project was opened
    ProjectOpened,
    /// A project was saved
    ProjectSaved,
    /// A project was closed
    ProjectClosed,
    /// Project settings were changed
    ProjectSettingsChanged,

    // Plugin events
    /// A plugin was loaded
    PluginLoaded,
    /// A plugin was unloaded
    PluginUnloaded,
    /// A plugin was enabled
    PluginEnabled,
    /// A plugin was disabled
    PluginDisabled,

    // UI events
    /// The UI layout was changed
    LayoutChanged,
    /// A panel was opened
    PanelOpened,
    /// A panel was closed
    PanelClosed,
    /// The theme was changed
    ThemeChanged,

    // Application events
    /// The application is starting up
    ApplicationStartup,
    /// The application is shutting down
    ApplicationShutdown,
    /// Configuration was changed
    ConfigurationChanged,

    // Custom events for plugins
    /// Custom event type for plugin-specific events
    Custom,
}

impl EventType {
    /// Get a human-readable description of the event type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::EventType;
    ///
    /// assert_eq!(EventType::DocumentChanged.description(), "Document content was changed");
    /// assert_eq!(EventType::PluginLoaded.description(), "Plugin was loaded");
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            EventType::DocumentChanged => "Document content was changed",
            EventType::DocumentCreated => "New document was created",
            EventType::DocumentOpened => "Document was opened",
            EventType::DocumentSaved => "Document was saved",
            EventType::DocumentClosed => "Document was closed",
            EventType::ProjectCreated => "Project was created",
            EventType::ProjectOpened => "Project was opened",
            EventType::ProjectSaved => "Project was saved",
            EventType::ProjectClosed => "Project was closed",
            EventType::ProjectSettingsChanged => "Project settings were changed",
            EventType::PluginLoaded => "Plugin was loaded",
            EventType::PluginUnloaded => "Plugin was unloaded",
            EventType::PluginEnabled => "Plugin was enabled",
            EventType::PluginDisabled => "Plugin was disabled",
            EventType::LayoutChanged => "UI layout was changed",
            EventType::PanelOpened => "Panel was opened",
            EventType::PanelClosed => "Panel was closed",
            EventType::ThemeChanged => "Theme was changed",
            EventType::ApplicationStartup => "Application is starting up",
            EventType::ApplicationShutdown => "Application is shutting down",
            EventType::ConfigurationChanged => "Configuration was changed",
            EventType::Custom => "Custom plugin event",
        }
    }

    /// Get all available event types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::EventType;
    ///
    /// let types = EventType::all();
    /// assert!(types.contains(&EventType::DocumentChanged));
    /// assert!(types.contains(&EventType::PluginLoaded));
    /// ```
    pub fn all() -> Vec<EventType> {
        vec![
            EventType::DocumentChanged,
            EventType::DocumentCreated,
            EventType::DocumentOpened,
            EventType::DocumentSaved,
            EventType::DocumentClosed,
            EventType::ProjectCreated,
            EventType::ProjectOpened,
            EventType::ProjectSaved,
            EventType::ProjectClosed,
            EventType::ProjectSettingsChanged,
            EventType::PluginLoaded,
            EventType::PluginUnloaded,
            EventType::PluginEnabled,
            EventType::PluginDisabled,
            EventType::LayoutChanged,
            EventType::PanelOpened,
            EventType::PanelClosed,
            EventType::ThemeChanged,
            EventType::ApplicationStartup,
            EventType::ApplicationShutdown,
            EventType::ConfigurationChanged,
            EventType::Custom,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        handled_events: Vec<EventType>,
    }

    impl EventHandler for TestHandler {
        fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
            self.handled_events.push(event.event_type());
            Ok(())
        }
    }

    #[test]
    fn test_event_creation() {
        let event = Event::new(EventType::DocumentChanged, "test data");
        
        assert_eq!(event.event_type(), EventType::DocumentChanged);
        assert_eq!(event.data(), "test data");
        assert!(event.metadata().is_empty());
        assert!(event.id() != Uuid::nil());
    }

    #[test]
    fn test_event_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("plugin".to_string(), "test-plugin".to_string());
        metadata.insert("version".to_string(), "1.0.0".to_string());

        let event = Event::with_metadata(EventType::PluginLoaded, "loaded", metadata);
        
        assert_eq!(event.event_type(), EventType::PluginLoaded);
        assert_eq!(event.data(), "loaded");
        assert_eq!(event.get_metadata("plugin"), Some("test-plugin"));
        assert_eq!(event.get_metadata("version"), Some("1.0.0"));
        assert_eq!(event.get_metadata("missing"), None);
    }

    #[test]
    fn test_event_metadata_modification() {
        let mut event = Event::new(EventType::Custom, "test");
        
        event.set_metadata("key1", "value1");
        event.set_metadata("key2", "value2");
        
        assert_eq!(event.get_metadata("key1"), Some("value1"));
        assert_eq!(event.get_metadata("key2"), Some("value2"));
        assert_eq!(event.metadata().len(), 2);
    }

    #[test]
    fn test_event_handler() {
        let mut handler = TestHandler {
            handled_events: Vec::new(),
        };

        let event1 = Event::new(EventType::DocumentChanged, "change1");
        let event2 = Event::new(EventType::PluginLoaded, "load1");

        assert!(handler.handle(&event1).is_ok());
        assert!(handler.handle(&event2).is_ok());

        assert_eq!(handler.handled_events.len(), 2);
        assert_eq!(handler.handled_events[0], EventType::DocumentChanged);
        assert_eq!(handler.handled_events[1], EventType::PluginLoaded);
    }

    #[test]
    fn test_event_type_description() {
        assert_eq!(
            EventType::DocumentChanged.description(),
            "Document content was changed"
        );
        assert_eq!(EventType::PluginLoaded.description(), "Plugin was loaded");
        assert_eq!(EventType::Custom.description(), "Custom plugin event");
    }

    #[test]
    fn test_event_type_all() {
        let types = EventType::all();
        assert!(types.len() > 10);
        assert!(types.contains(&EventType::DocumentChanged));
        assert!(types.contains(&EventType::ApplicationStartup));
        assert!(types.contains(&EventType::Custom));
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::new(EventType::DocumentChanged, "test serialization");
        
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        
        assert_eq!(event.id(), deserialized.id());
        assert_eq!(event.event_type(), deserialized.event_type());
        assert_eq!(event.data(), deserialized.data());
        assert_eq!(event.timestamp(), deserialized.timestamp());
    }
}