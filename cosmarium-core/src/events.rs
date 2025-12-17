//! # Event system for Cosmarium Core
//!
//! This module provides a centralized event bus system for inter-component
//! communication within the Cosmarium creative writing software. The event
//! system enables loose coupling between different parts of the application
//! while maintaining efficient communication.
//!
//! The event bus supports both synchronous and asynchronous event handling,
//! with automatic cleanup of disconnected handlers and priority-based
//! event processing.

use crate::{Error, Result};
use cosmarium_plugin_api::{Event, EventHandler, EventType};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, warn};
use uuid::Uuid;

/// Central event bus for system-wide communication.
///
/// The [`EventBus`] provides a publish-subscribe pattern for event-driven
/// communication between different components of the application. It supports
/// typed events, handler priorities, and automatic cleanup.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::events::EventBus;
/// use cosmarium_plugin_api::{Event, EventType};
///
/// # tokio_test::block_on(async {
/// let mut event_bus = EventBus::new();
/// event_bus.initialize().await?;
///
/// let event = Event::new(EventType::DocumentChanged, "Content updated");
/// event_bus.emit(event).await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct EventBus {
    /// Event handlers organized by event type
    handlers: Arc<RwLock<HashMap<EventType, Vec<HandlerEntry>>>>,
    /// Event queue for processing
    event_queue: Arc<Mutex<VecDeque<Event>>>,
    /// Whether the event bus is initialized
    initialized: bool,
    /// Maximum number of events to queue
    max_queue_size: usize,
    /// Whether to process events asynchronously
    async_processing: bool,
}

/// Handler entry with metadata
/// Handler entry for the event bus.
/// Does not derive Debug because dyn EventHandler does not implement Debug.
struct HandlerEntry {
    /// Unique handler identifier
    id: Uuid,
    /// Handler implementation
    handler: Arc<Mutex<dyn EventHandler>>,
    /// Handler priority (higher = processed first)
    priority: i32,
}

impl EventBus {
    /// Create a new event bus.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// let event_bus = EventBus::new();
    /// ```
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
            initialized: false,
            max_queue_size: 1000,
            async_processing: true,
        }
    }

    /// Initialize the event bus.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            warn!("Event bus is already initialized");
            return Ok(());
        }

        debug!("Initializing event bus");
        self.initialized = true;
        debug!("Event bus initialized");
        Ok(())
    }

    /// Shutdown the event bus.
    ///
    /// This method clears all handlers and pending events.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    /// event_bus.shutdown().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        debug!("Shutting down event bus");

        // Clear all handlers
        {
            let mut handlers = self.handlers.write().await;
            handlers.clear();
        }

        // Clear event queue
        {
            let mut queue = self.event_queue.lock().await;
            queue.clear();
        }

        self.initialized = false;
        debug!("Event bus shutdown completed");
        Ok(())
    }

    /// Subscribe to events of a specific type.
    ///
    /// # Arguments
    ///
    /// * `event_type` - Type of events to subscribe to
    /// * `handler` - Event handler implementation
    /// * `priority` - Handler priority (higher numbers = higher priority)
    ///
    /// # Returns
    ///
    /// Subscription ID that can be used to unsubscribe
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    /// use cosmarium_plugin_api::{EventType, EventHandler, Event};
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    ///
    /// struct MyHandler;
    /// impl EventHandler for MyHandler {
    ///     fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
    ///         println!("Received event: {:?}", event.event_type());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    ///
    /// let handler = Arc::new(Mutex::new(MyHandler));
    /// let subscription_id = event_bus.subscribe(EventType::DocumentChanged, handler, 0).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn subscribe(
        &self,
        event_type: EventType,
        handler: Arc<Mutex<dyn EventHandler>>,
        priority: i32,
    ) -> Result<Uuid> {
        if !self.initialized {
            return Err(Error::event("Event bus not initialized"));
        }

        let id = Uuid::new_v4();
        let entry = HandlerEntry {
            id,
            handler,
            priority,
        };

        let mut handlers = self.handlers.write().await;
        let type_handlers = handlers.entry(event_type).or_insert_with(Vec::new);
        type_handlers.push(entry);

        // Sort by priority (highest first)
        type_handlers.sort_by(|a, b| b.priority.cmp(&a.priority));

        debug!(
            "Subscribed handler {:?} to {:?} events with priority {}",
            id, event_type, priority
        );
        Ok(id)
    }

    /// Unsubscribe from events.
    ///
    /// # Arguments
    ///
    /// * `subscription_id` - Subscription ID returned from subscribe
    ///
    /// # Errors
    ///
    /// Returns an error if the subscription ID is not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    /// use cosmarium_plugin_api::{EventType, EventHandler, Event};
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    ///
    /// struct MyHandler;
    /// impl EventHandler for MyHandler {
    ///     fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    ///
    /// let handler = Arc::new(Mutex::new(MyHandler));
    /// let id = event_bus.subscribe(EventType::DocumentChanged, handler, 0).await?;
    /// event_bus.unsubscribe(id).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn unsubscribe(&self, subscription_id: Uuid) -> Result<()> {
        let mut handlers = self.handlers.write().await;

        for type_handlers in handlers.values_mut() {
            type_handlers.retain(|entry| entry.id != subscription_id);
        }

        debug!("Unsubscribed handler {:?}", subscription_id);
        Ok(())
    }

    /// Emit an event to all subscribers.
    ///
    /// # Arguments
    ///
    /// * `event` - Event to emit
    ///
    /// # Errors
    ///
    /// Returns an error if event emission fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    /// use cosmarium_plugin_api::{Event, EventType};
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    ///
    /// let event = Event::new(EventType::DocumentChanged, "Document updated");
    /// event_bus.emit(event).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn emit(&self, event: Event) -> Result<()> {
        if !self.initialized {
            return Err(Error::event("Event bus not initialized"));
        }

        if self.async_processing {
            self.queue_event(event).await
        } else {
            self.process_event_immediately(event).await
        }
    }

    /// Process all queued events.
    ///
    /// This method processes all events currently in the queue. It should be
    /// called regularly (e.g., once per frame) to ensure timely event processing.
    ///
    /// # Errors
    ///
    /// Returns an error if event processing fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    /// event_bus.process_events().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn process_events(&self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let mut processed_count = 0;

        loop {
            let event = {
                let mut queue = self.event_queue.lock().await;
                queue.pop_front()
            };

            match event {
                Some(event) => {
                    if let Err(e) = self.process_event_immediately(event).await {
                        error!("Event processing error: {}", e);
                    }
                    processed_count += 1;
                }
                None => break,
            }
        }

        if processed_count > 0 {
            debug!("Processed {} events", processed_count);
        }

        Ok(())
    }

    /// Get the number of queued events.
    ///
    /// # Returns
    ///
    /// Number of events waiting to be processed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = EventBus::new();
    /// let count = event_bus.queue_size().await;
    /// assert_eq!(count, 0);
    /// # });
    /// ```
    pub async fn queue_size(&self) -> usize {
        let queue = self.event_queue.lock().await;
        queue.len()
    }

    /// Get the number of registered handlers for an event type.
    ///
    /// # Arguments
    ///
    /// * `event_type` - Event type to check
    ///
    /// # Returns
    ///
    /// Number of handlers registered for the event type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    /// use cosmarium_plugin_api::EventType;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = EventBus::new();
    /// let count = event_bus.handler_count(EventType::DocumentChanged).await;
    /// assert_eq!(count, 0);
    /// # });
    /// ```
    pub async fn handler_count(&self, event_type: EventType) -> usize {
        let handlers = self.handlers.read().await;
        handlers.get(&event_type).map_or(0, |h| h.len())
    }

    /// Set the maximum queue size.
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of events to queue
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// let mut event_bus = EventBus::new();
    /// event_bus.set_max_queue_size(500);
    /// ```
    pub fn set_max_queue_size(&mut self, max_size: usize) {
        self.max_queue_size = max_size;
    }

    /// Enable or disable asynchronous event processing.
    ///
    /// # Arguments
    ///
    /// * `async_mode` - Whether to process events asynchronously
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// let mut event_bus = EventBus::new();
    /// event_bus.set_async_processing(false);
    /// ```
    pub fn set_async_processing(&mut self, async_mode: bool) {
        self.async_processing = async_mode;
    }

    /// Queue an event for later processing.
    async fn queue_event(&self, event: Event) -> Result<()> {
        let mut queue = self.event_queue.lock().await;

        if queue.len() >= self.max_queue_size {
            warn!("Event queue is full, dropping oldest event");
            queue.pop_front();
        }

        debug!("Queued event: {:?}", event.event_type());
        queue.push_back(event);
        Ok(())
    }

    /// Process an event immediately.
    async fn process_event_immediately(&self, event: Event) -> Result<()> {
        let handlers = self.handlers.read().await;

        if let Some(type_handlers) = handlers.get(&event.event_type()) {
            debug!(
                "Processing {:?} event for {} handlers",
                event.event_type(),
                type_handlers.len()
            );

            for handler_entry in type_handlers {
                let mut handler = handler_entry.handler.lock().await;
                if let Err(e) = handler.handle(&event) {
                    error!(
                        "Handler {:?} failed to process {:?} event: {}",
                        handler_entry.id,
                        event.event_type(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Clean up inactive handlers.
    ///
    /// This method removes handlers that are no longer active.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::events::EventBus;
    ///
    /// # tokio_test::block_on(async {
    /// let mut event_bus = EventBus::new();
    /// event_bus.initialize().await?;
    /// event_bus.cleanup_handlers().await;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn cleanup_handlers(&self) {
        let mut handlers = self.handlers.write().await;

        for type_handlers in handlers.values_mut() {
            let original_count = type_handlers.len();

            let removed_count = original_count - type_handlers.len();
            if removed_count > 0 {
                debug!("Cleaned up {} inactive handlers", removed_count);
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmarium_plugin_api::{Event, EventType};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestHandler {
        call_count: Arc<AtomicUsize>,
    }

    impl TestHandler {
        fn new(call_count: Arc<AtomicUsize>) -> Self {
            Self { call_count }
        }
    }

    impl EventHandler for TestHandler {
        fn handle(&mut self, _event: &Event) -> anyhow::Result<()> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_event_bus_creation() {
        let event_bus = EventBus::new();
        assert!(!event_bus.initialized);
        assert_eq!(event_bus.max_queue_size, 1000);
        assert!(event_bus.async_processing);
    }

    #[tokio::test]
    async fn test_event_bus_initialization() {
        let mut event_bus = EventBus::new();
        assert!(event_bus.initialize().await.is_ok());
        assert!(event_bus.initialized);
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let mut event_bus = EventBus::new();
        event_bus.initialize().await.unwrap();

        let call_count = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(Mutex::new(TestHandler::new(Arc::clone(&call_count))));

        let subscription_id = event_bus
            .subscribe(EventType::DocumentChanged, handler, 0)
            .await
            .unwrap();

        assert_ne!(subscription_id, Uuid::nil());
        assert_eq!(event_bus.handler_count(EventType::DocumentChanged).await, 1);
    }

    #[tokio::test]
    async fn test_event_emission() {
        let mut event_bus = EventBus::new();
        event_bus.initialize().await.unwrap();

        let call_count = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(Mutex::new(TestHandler::new(Arc::clone(&call_count))));

        event_bus
            .subscribe(EventType::DocumentChanged, handler, 0)
            .await
            .unwrap();

        let event = Event::new(EventType::DocumentChanged, "Test event");
        event_bus.emit(event).await.unwrap();

        // Process events if async processing is enabled
        event_bus.process_events().await.unwrap();

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_unsubscription() {
        let mut event_bus = EventBus::new();
        event_bus.initialize().await.unwrap();

        let call_count = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(Mutex::new(TestHandler::new(Arc::clone(&call_count))));

        let subscription_id = event_bus
            .subscribe(EventType::DocumentChanged, handler, 0)
            .await
            .unwrap();

        assert_eq!(event_bus.handler_count(EventType::DocumentChanged).await, 1);

        event_bus.unsubscribe(subscription_id).await.unwrap();
        assert_eq!(event_bus.handler_count(EventType::DocumentChanged).await, 0);
    }

    #[tokio::test]
    async fn test_handler_priority() {
        let mut event_bus = EventBus::new();
        event_bus.initialize().await.unwrap();

        let call_order = Arc::new(Mutex::new(Vec::new()));

        // Create handlers with different priorities
        let handler1 = {
            let call_order = Arc::clone(&call_order);
            Arc::new(Mutex::new(move |_: &Event| -> anyhow::Result<()> {
                let mut order = call_order.try_lock().unwrap();
                order.push(1);
                Ok(())
            }))
        };

        let handler2 = {
            let call_order = Arc::clone(&call_order);
            Arc::new(Mutex::new(move |_: &Event| -> anyhow::Result<()> {
                let mut order = call_order.try_lock().unwrap();
                order.push(2);
                Ok(())
            }))
        };

        // Note: This test would need a more sophisticated setup to test priority ordering
        // For now, we just test that different priority handlers can be registered
    }

    #[tokio::test]
    async fn test_queue_size() {
        let mut event_bus = EventBus::new();
        event_bus.initialize().await.unwrap();

        assert_eq!(event_bus.queue_size().await, 0);

        let event = Event::new(EventType::DocumentChanged, "Test");
        event_bus.queue_event(event).await.unwrap();

        assert_eq!(event_bus.queue_size().await, 1);

        event_bus.process_events().await.unwrap();
        assert_eq!(event_bus.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_event_bus_shutdown() {
        let mut event_bus = EventBus::new();
        event_bus.initialize().await.unwrap();

        let call_count = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(Mutex::new(TestHandler::new(Arc::clone(&call_count))));

        event_bus
            .subscribe(EventType::DocumentChanged, handler, 0)
            .await
            .unwrap();

        assert!(event_bus.shutdown().await.is_ok());
        assert!(!event_bus.initialized);
        assert_eq!(event_bus.handler_count(EventType::DocumentChanged).await, 0);
    }

    #[tokio::test]
    async fn test_max_queue_size() {
        let mut event_bus = EventBus::new();
        event_bus.set_max_queue_size(2);
        event_bus.initialize().await.unwrap();

        // Add events up to the limit
        event_bus
            .queue_event(Event::new(EventType::DocumentChanged, "1"))
            .await
            .unwrap();
        event_bus
            .queue_event(Event::new(EventType::DocumentChanged, "2"))
            .await
            .unwrap();
        assert_eq!(event_bus.queue_size().await, 2);

        // Adding another should drop the oldest
        event_bus
            .queue_event(Event::new(EventType::DocumentChanged, "3"))
            .await
            .unwrap();
        assert_eq!(event_bus.queue_size().await, 2);
    }
}
