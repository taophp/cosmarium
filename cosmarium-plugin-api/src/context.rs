//! Plugin context module providing shared state and services to plugins.
//!
//! The [`PluginContext`] is the main interface through which plugins interact
//! with the Cosmarium core and other plugins. It provides access to shared state,
//! event system, configuration, and other core services.

use crate::{Event, EventHandler};
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Context object providing plugins access to core services and shared state.
///
/// The plugin context acts as the main communication channel between plugins
/// and the Cosmarium core. It provides access to:
/// - Shared state management
/// - Event system for inter-plugin communication
/// - Configuration and settings
/// - Document and project access
/// - UI layout management
///
/// # Example
///
/// ```rust
/// use cosmarium_plugin_api::{PluginContext, Event, EventType};
///
/// fn my_plugin_function(ctx: &mut PluginContext) -> anyhow::Result<()> {
///     // Access shared state
///     let count: i32 = ctx.get_shared_state("word_count").unwrap_or(0);
///     ctx.set_shared_state("word_count", count + 100);
///
///     // Send an event
///     let event = Event::new(EventType::DocumentChanged, "Document updated".to_string());
///     ctx.emit_event(event);
///
///     Ok(())
/// }
/// ```
pub struct PluginContext {
    /// Shared state storage accessible to all plugins
    shared_state: Arc<RwLock<SharedState>>,
    /// Event handlers registered by plugins
    event_handlers: HashMap<String, Vec<Box<dyn EventHandler>>>,
    /// Application configuration
    config: HashMap<String, serde_json::Value>,
    /// Plugin-specific data storage
    plugin_data: HashMap<String, HashMap<String, Box<dyn Any + Send + Sync>>>,
    /// Path to the currently active project (if any)
    project_path: Arc<RwLock<Option<std::path::PathBuf>>>,
}

impl PluginContext {
    /// Create a new plugin context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let ctx = PluginContext::new();
    /// ```
    pub fn new() -> Self {
        Self {
            shared_state: Arc::new(RwLock::new(SharedState::new())),
            event_handlers: HashMap::new(),
            config: HashMap::new(),
            plugin_data: HashMap::new(),
            project_path: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a value from shared state.
    ///
    /// Returns the value if it exists and can be downcast to the requested type,
    /// otherwise returns None.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let mut ctx = PluginContext::new();
    /// ctx.set_shared_state("count", 42i32);
    /// let count: Option<i32> = ctx.get_shared_state("count");
    /// assert_eq!(count, Some(42));
    /// ```
    pub fn get_shared_state<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let state = self.shared_state.read().ok()?;
        state.get(key)
    }

    /// Set a value in shared state.
    ///
    /// The value must implement Clone and have a static lifetime.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let mut ctx = PluginContext::new();
    /// ctx.set_shared_state("message", "Hello, World!".to_string());
    /// ```
    pub fn set_shared_state<T: Clone + Send + Sync + 'static>(&mut self, key: &str, value: T) {
        if let Ok(mut state) = self.shared_state.write() {
            state.set(key, value);
        }
    }

    /// Emit an event to be handled by registered handlers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{PluginContext, Event, EventType};
    ///
    /// let mut ctx = PluginContext::new();
    /// let event = Event::new(EventType::DocumentChanged, "Content updated".to_string());
    /// ctx.emit_event(event);
    /// ```
    pub fn emit_event(&mut self, event: Event) {
        let event_type = format!("{:?}", event.event_type());
        if let Some(handlers) = self.event_handlers.get_mut(&event_type) {
            for handler in handlers.iter_mut() {
                if let Err(e) = handler.handle(&event) {
                    tracing::error!("Error handling event {}: {}", event_type, e);
                }
            }
        }
    }

    /// Register an event handler for a specific event type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::{PluginContext, Event, EventHandler, EventType};
    ///
    /// struct MyHandler;
    ///
    /// impl EventHandler for MyHandler {
    ///     fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
    ///         println!("Received event: {:?}", event.event_type());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut ctx = PluginContext::new();
    /// ctx.register_event_handler("DocumentChanged", Box::new(MyHandler));
    /// ```
    pub fn register_event_handler<S: Into<String>>(
        &mut self,
        event_type: S,
        handler: Box<dyn EventHandler>,
    ) {
        let event_type = event_type.into();
        self.event_handlers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// Get a configuration value.
    ///
    /// Returns the configuration value if it exists and can be deserialized
    /// to the requested type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let ctx = PluginContext::new();
    /// let font_size: Option<f32> = ctx.get_config("ui.font_size");
    /// ```
    pub fn get_config<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        let value = self.config.get(key)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Set a configuration value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let mut ctx = PluginContext::new();
    /// ctx.set_config("ui.font_size", 14.0f32);
    /// ```
    pub fn set_config<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.config.insert(key.to_string(), json_value);
        }
    }

    /// Store plugin-specific data.
    ///
    /// This allows plugins to store arbitrary data that persists across
    /// plugin lifecycle events.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let mut ctx = PluginContext::new();
    /// ctx.set_plugin_data("my-plugin", "counter", 42i32);
    /// let counter: Option<i32> = ctx.get_plugin_data("my-plugin", "counter");
    /// assert_eq!(counter, Some(42));
    /// ```
    pub fn set_plugin_data<T: Send + Sync + 'static>(
        &mut self,
        plugin_name: &str,
        key: &str,
        value: T,
    ) {
        self.plugin_data
            .entry(plugin_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), Box::new(value));
    }

    /// Retrieve plugin-specific data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::PluginContext;
    ///
    /// let mut ctx = PluginContext::new();
    /// ctx.set_plugin_data("my-plugin", "data", "hello".to_string());
    /// let data: Option<String> = ctx.get_plugin_data("my-plugin", "data");
    /// assert_eq!(data, Some("hello".to_string()));
    /// ```
    pub fn get_plugin_data<T: Clone + 'static>(&self, plugin_name: &str, key: &str) -> Option<T> {
        let plugin_data = self.plugin_data.get(plugin_name)?;
        let value = plugin_data.get(key)?;
        let any_value = value.as_ref();
        any_value.downcast_ref::<T>().cloned()
    }

    /// Get access to the shared state for advanced operations.
    ///
    /// Returns an Arc to the shared state for scenarios where plugins need
    /// to perform more complex state operations or share state handles.
    pub fn shared_state(&self) -> Arc<RwLock<SharedState>> {
        Arc::clone(&self.shared_state)
    }

    /// Set the currently active project path.
    pub fn set_project_path(&self, path: Option<std::path::PathBuf>) {
        if let Ok(mut lock) = self.project_path.write() {
            *lock = path;
        }
    }

    /// Get the currently active project path.
    pub fn project_path(&self) -> Option<std::path::PathBuf> {
        self.project_path.read().ok().and_then(|lock| lock.clone())
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared state container for data accessible across all plugins.
///
/// The shared state uses type-safe storage and retrieval, allowing plugins
/// to share data without needing to know about each other's internal structure.
pub struct SharedState {
    /// Storage for typed values indexed by string keys
    data: HashMap<String, Box<dyn Any + Send + Sync>>,
    /// Type information for stored values
    types: HashMap<String, TypeId>,
}

impl SharedState {
    /// Create a new shared state container.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::SharedState;
    ///
    /// let state = SharedState::new();
    /// ```
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            types: HashMap::new(),
        }
    }

    /// Store a value with the given key.
    ///
    /// The value must implement Clone, Send, and Sync with a static lifetime.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::SharedState;
    ///
    /// let mut state = SharedState::new();
    /// state.set("counter", 42i32);
    /// ```
    pub fn set<T: Clone + Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.types.insert(key.to_string(), TypeId::of::<T>());
        self.data.insert(key.to_string(), Box::new(value));
    }

    /// Retrieve a value with the given key and type.
    ///
    /// Returns Some(value) if the key exists and the type matches,
    /// otherwise returns None.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::SharedState;
    ///
    /// let mut state = SharedState::new();
    /// state.set("message", "Hello".to_string());
    /// let message: Option<String> = state.get("message");
    /// assert_eq!(message, Some("Hello".to_string()));
    /// ```
    pub fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let expected_type = TypeId::of::<T>();
        let stored_type = self.types.get(key)?;

        if *stored_type != expected_type {
            return None;
        }

        let value = self.data.get(key)?;
        let any_value = value.as_ref();
        any_value.downcast_ref::<T>().cloned()
    }

    /// Check if a key exists in the shared state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::SharedState;
    ///
    /// let mut state = SharedState::new();
    /// state.set("key", 123);
    /// assert!(state.contains_key("key"));
    /// assert!(!state.contains_key("missing"));
    /// ```
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Remove a value from the shared state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::SharedState;
    ///
    /// let mut state = SharedState::new();
    /// state.set("temp", 42);
    /// assert!(state.contains_key("temp"));
    /// state.remove("temp");
    /// assert!(!state.contains_key("temp"));
    /// ```
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
        self.types.remove(key);
    }

    /// Get all keys in the shared state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_plugin_api::SharedState;
    ///
    /// let mut state = SharedState::new();
    /// state.set("key1", 1);
    /// state.set("key2", 2);
    /// let keys: Vec<&String> = state.keys().collect();
    /// assert_eq!(keys.len(), 2);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Event, EventType};

    #[test]
    fn test_shared_state_basic_operations() {
        let mut state = SharedState::new();

        // Test set and get
        state.set("number", 42i32);
        assert_eq!(state.get::<i32>("number"), Some(42));

        // Test type safety
        assert_eq!(state.get::<String>("number"), None);

        // Test contains_key
        assert!(state.contains_key("number"));
        assert!(!state.contains_key("missing"));

        // Test remove
        state.remove("number");
        assert!(!state.contains_key("number"));
    }

    #[test]
    fn test_plugin_context_shared_state() {
        let mut ctx = PluginContext::new();

        ctx.set_shared_state("test", "value".to_string());
        assert_eq!(
            ctx.get_shared_state::<String>("test"),
            Some("value".to_string())
        );
        assert_eq!(ctx.get_shared_state::<i32>("test"), None);
    }

    #[test]
    fn test_plugin_context_config() {
        let mut ctx = PluginContext::new();

        ctx.set_config("font_size", 14.0f32);
        assert_eq!(ctx.get_config::<f32>("font_size"), Some(14.0));
        assert_eq!(ctx.get_config::<i32>("font_size"), None);
    }

    #[test]
    fn test_plugin_context_plugin_data() {
        let mut ctx = PluginContext::new();

        ctx.set_plugin_data("my-plugin", "counter", 100i32);
        assert_eq!(
            ctx.get_plugin_data::<i32>("my-plugin", "counter"),
            Some(100)
        );
        assert_eq!(ctx.get_plugin_data::<i32>("other-plugin", "counter"), None);
        assert_eq!(ctx.get_plugin_data::<String>("my-plugin", "counter"), None);
    }
}
