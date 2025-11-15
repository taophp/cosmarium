//! # Document management system for Cosmarium Core
//!
//! This module provides document handling capabilities for the Cosmarium
//! creative writing software. It manages document lifecycle, content storage,
//! versioning, and metadata for text-based documents used in writing projects.
//!
//! The document system supports multiple formats (Markdown, plain text, etc.)
//! and provides automatic backup, change tracking, and collaborative editing
//! features.

use crate::{Error, Result, events::EventBus};
use cosmarium_plugin_api::{Event, EventType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Document management system for Cosmarium.
///
/// The [`DocumentManager`] handles all document-related operations including
/// creation, loading, saving, and metadata management. It provides a unified
/// interface for working with different document types and formats.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::document::DocumentManager;
/// use cosmarium_core::events::EventBus;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// # tokio_test::block_on(async {
/// let event_bus = Arc::new(RwLock::new(EventBus::new()));
/// let mut manager = DocumentManager::new();
/// manager.initialize(event_bus).await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct DocumentManager {
    /// Currently open documents
    documents: HashMap<Uuid, Document>,
    /// Document metadata cache
    metadata_cache: HashMap<Uuid, DocumentMetadata>,
    /// Event bus for system communication
    event_bus: Option<Arc<RwLock<EventBus>>>,
    /// Auto-save interval
    auto_save_interval: Duration,
    /// Whether the manager is initialized
    initialized: bool,
    /// Maximum number of concurrent documents
    max_documents: usize,
}

impl DocumentManager {
    /// Create a new document manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::document::DocumentManager;
    ///
    /// let manager = DocumentManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            metadata_cache: HashMap::new(),
            event_bus: None,
            auto_save_interval: Duration::from_secs(30),
            initialized: false,
            max_documents: 100,
        }
    }

    /// Initialize the document manager.
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
    /// use cosmarium_core::{document::DocumentManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn initialize(&mut self, event_bus: Arc<RwLock<EventBus>>) -> Result<()> {
        if self.initialized {
            warn!("Document manager is already initialized");
            return Ok(());
        }

        info!("Initializing document manager");
        self.event_bus = Some(event_bus);
        self.initialized = true;
        info!("Document manager initialized");
        Ok(())
    }

    /// Shutdown the document manager.
    ///
    /// This method saves all open documents and cleans up resources.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.shutdown().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        info!("Shutting down document manager");

        // Save all open documents
        //
        // Extract the IDs first so we don't hold any borrows on `self.documents`
        // across await points. For each document we extract the minimal owned
        // data needed (title, content, owned path) before performing async I/O.
        // After the write succeeds we mark the document saved and emit an event.
        let ids_to_save: Vec<_> = self.documents.iter()
            .filter(|(_, doc)| doc.has_unsaved_changes())
            .map(|(id, _)| *id)
            .collect();

        for id in ids_to_save {
            // Extract the data we need without holding long-lived borrows.
            // This avoids mutable/immutable borrow conflicts across awaits.
            let (title, content, path_opt) = {
                let doc_ref = self.documents.get(&id).unwrap();
                // Clone/own the values we need for the write operation.
                (doc_ref.title().to_string(), doc_ref.content().to_string(), doc_ref.file_path().map(|p| p.to_path_buf()))
            };

            if let Some(path) = path_opt {
                // Perform the file write outside of any borrows to self.
                match tokio::fs::write(&path, content).await {
                    Ok(_) => {
                        // Mark the document saved using a short mutable borrow.
                        if let Some(doc_mut) = self.documents.get_mut(&id) {
                            doc_mut.mark_saved();
                        }

                        debug!("Saved document '{}' to {:?}", title, path);

                        // Emit document saved event after successful save.
                        if let Some(ref event_bus) = self.event_bus {
                            let bus = event_bus.write().await;
                            let event = Event::new(EventType::DocumentSaved, format!("Saved document: {}", title));
                            let _ = bus.emit(event).await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to write document '{}': {}", title, e);
                    }
                }
            } else {
                error!("Failed to save document '{}': no file path", title);
            }
        }

        // Clear caches and mark uninitialized after all saves and events are done.
        self.documents.clear();
        self.metadata_cache.clear();
        self.initialized = false;

        info!("Document manager shutdown completed");
        Ok(())
    }

    /// Create a new document.
    ///
    /// # Arguments
    ///
    /// * `title` - Document title
    /// * `content` - Initial document content
    /// * `format` - Document format
    ///
    /// # Returns
    ///
    /// The ID of the newly created document.
    ///
    /// # Errors
    ///
    /// Returns an error if document creation fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus};
    /// use cosmarium_core::document::DocumentFormat;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let doc_id = manager.create_document(
    ///     "My Novel",
    ///     "# Chapter 1\n\nIt was a dark and stormy night...",
    ///     DocumentFormat::Markdown
    /// ).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn create_document(
        &mut self,
        title: &str,
        content: &str,
        format: DocumentFormat,
    ) -> Result<Uuid> {
        if !self.initialized {
            return Err(Error::document("Document manager not initialized"));
        }

        if self.documents.len() >= self.max_documents {
            return Err(Error::document("Maximum number of documents reached"));
        }

        let id = Uuid::new_v4();
        let document = Document::new(id, title, content, format);

        self.documents.insert(id, document);

        // Emit document created event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::DocumentCreated, format!("Created document: {}", title));
            let _ = bus.emit(event).await;
        }

        info!("Created new document '{}' with ID {}", title, id);
        Ok(id)
    }

    /// Open a document from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the document file
    ///
    /// # Returns
    ///
    /// The ID of the opened document.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be opened.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cosmarium_core::{document::DocumentManager, events::EventBus};
    /// use std::sync::Arc;
    /// use std::path::Path;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let doc_id = manager.open_document(Path::new("my_document.md")).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn open_document<P: AsRef<Path>>(&mut self, path: P) -> Result<Uuid> {
        let path = path.as_ref();
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| Error::document(format!("Failed to read document: {}", e)))?;

        let format = DocumentFormat::from_extension(path.extension().and_then(|s| s.to_str()));
        let title = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let id = Uuid::new_v4();
        let mut document = Document::new(id, &title, &content, format);
        document.set_file_path(path);
        document.mark_saved(); // File was just loaded, so it's saved

        self.documents.insert(id, document);

        // Emit document opened event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::DocumentOpened, format!("Opened document: {}", title));
            let _ = bus.emit(event).await;
        }

        info!("Opened document '{}' from {:?} with ID {}", title, path, id);
        Ok(id)
    }

    /// Save a document.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document to save
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be saved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus, document::DocumentFormat};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let doc_id = manager.create_document("Test", "Content", DocumentFormat::Markdown).await?;
    /// // manager.save_document(doc_id).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    /// Save a document to disk and emit a saved event.
    ///
    /// This function extracts the minimal required information from the
    /// in-memory document, performs the I/O operation, marks the document as
    /// saved, and only then emits the DocumentSaved event.
    ///
    /// # Errors
    ///
    /// Returns an error if the document does not exist or if writing fails.
    pub async fn save_document(&mut self, document_id: Uuid) -> Result<()> {
        // Extract required data while holding a short borrow.
        // Clone the file path into an owned PathBuf to avoid holding a borrow
        // across the await point below.
        let (title, content, path_opt) = {
            let document = self.documents.get_mut(&document_id)
                .ok_or_else(|| Error::document("Document not found"))?;
            (
                document.title().to_string(),
                document.content().to_string(),
                document.file_path().map(|p| p.to_path_buf()),
            )
        };

        // Perform file write outside of any long-lived mutable borrow.
        if let Some(path) = path_opt {
            tokio::fs::write(&path, content).await
                .map_err(|e| Error::document(format!("Failed to write document: {}", e)))?;

            // Mark the document saved now that the data has been persisted.
            if let Some(document) = self.documents.get_mut(&document_id) {
                document.mark_saved();
            }

            debug!("Saved document '{}' to {:?}", title, path);

            // Emit document saved event after successful save.
            if let Some(ref event_bus) = self.event_bus {
                let bus = event_bus.write().await;
                let event = Event::new(EventType::DocumentSaved, format!("Saved document: {}", title));
                let _ = bus.emit(event).await;
            }

            return Ok(());
        }

        Err(Error::document("Document has no file path"))
    }

    /// Get a document by ID.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document
    ///
    /// # Returns
    ///
    /// Reference to the document if found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus, document::DocumentFormat};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let doc_id = manager.create_document("Test", "Content", DocumentFormat::Markdown).await?;
    /// let document = manager.get_document(doc_id).unwrap();
    /// assert_eq!(document.title(), "Test");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn get_document(&self, document_id: Uuid) -> Option<&Document> {
        self.documents.get(&document_id)
    }

    /// Get a mutable document by ID.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document
    ///
    /// # Returns
    ///
    /// Mutable reference to the document if found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus, document::DocumentFormat};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let doc_id = manager.create_document("Test", "Content", DocumentFormat::Markdown).await?;
    /// let document = manager.get_document_mut(doc_id).unwrap();
    /// document.set_content("New content");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn get_document_mut(&mut self, document_id: Uuid) -> Option<&mut Document> {
        self.documents.get_mut(&document_id)
    }

    /// Close a document.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document to close
    /// * `save_if_modified` - Whether to save the document if it has unsaved changes
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be closed or saved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus, document::DocumentFormat};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let doc_id = manager.create_document("Test", "Content", DocumentFormat::Markdown).await?;
    /// manager.close_document(doc_id, true).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn close_document(&mut self, document_id: Uuid, save_if_modified: bool) -> Result<()> {
        let document = self.documents.get(&document_id)
            .ok_or_else(|| Error::document("Document not found"))?;

        let title = document.title().to_string();

        if save_if_modified && document.has_unsaved_changes() {
            self.save_document(document_id).await?;
        }

        self.documents.remove(&document_id);

        // Emit document closed event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::DocumentClosed, format!("Closed document: {}", title));
            let _ = bus.emit(event).await;
        }

        info!("Closed document '{}' (ID: {})", title, document_id);
        Ok(())
    }

    /// List all open documents.
    ///
    /// # Returns
    ///
    /// Vector of document IDs that are currently open.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus, document::DocumentFormat};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let docs = manager.list_documents();
    /// assert!(docs.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn list_documents(&self) -> Vec<Uuid> {
        self.documents.keys().cloned().collect()
    }

    /// Update method called regularly to handle auto-save and maintenance.
    ///
    /// # Errors
    ///
    /// Returns an error if update operations fail.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{document::DocumentManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = DocumentManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.update().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn update(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Check for documents that need auto-saving
        let now = SystemTime::now();
        let mut documents_to_save = Vec::new();

        for (id, document) in &self.documents {
            if document.has_unsaved_changes() {
                if let Ok(elapsed) = now.duration_since(document.last_modified_time()) {
                    if elapsed >= self.auto_save_interval {
                        documents_to_save.push(*id);
                    }
                }
            }
        }

        // Perform auto-saves
        for document_id in documents_to_save {
            if let Err(e) = self.save_document(document_id).await {
                error!("Auto-save failed for document {}: {}", document_id, e);
            } else {
                debug!("Auto-saved document {}", document_id);
            }
        }

        Ok(())
    }

    /// Internal method to save a document.
    ///
    /// This implementation extracts the minimal required information from the
    /// provided `document`, performs the I/O operation outside of any long-lived
    /// borrows, then marks the document as saved using the mutable reference.
    async fn save_document_internal(&mut self, document: &mut Document) -> Result<()> {
        // Extract data needed for the write operation while holding a short borrow.
        let path_opt = document.file_path().map(|p| p.to_path_buf());
        let content = document.content().to_string();
        let title = document.title().to_string();

        // Perform the write outside of any other borrows.
        if let Some(path) = path_opt {
            tokio::fs::write(&path, content).await
                .map_err(|e| Error::document(format!("Failed to write document: {}", e)))?;

            // Now mark the document saved using the mutable reference we already have.
            document.mark_saved();
            debug!("Saved document '{}' to {:?}", title, path);

            Ok(())
        } else {
            Err(Error::document("Document has no file path"))
        }
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A document in the Cosmarium system.
///
/// Documents represent individual text files or content units within a project.
/// They track content, metadata, and state information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document identifier
    id: Uuid,
    /// Document title
    title: String,
    /// Document content
    content: String,
    /// Document format
    format: DocumentFormat,
    /// File path (if saved to disk)
    file_path: Option<PathBuf>,
    /// Creation time
    created_at: SystemTime,
    /// Last modification time
    modified_at: SystemTime,
    /// Whether the document has unsaved changes
    has_unsaved_changes: bool,
    /// Document metadata
    metadata: DocumentMetadata,
}

impl Document {
    /// Create a new document.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique document identifier
    /// * `title` - Document title
    /// * `content` - Initial content
    /// * `format` - Document format
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::document::{Document, DocumentFormat};
    /// use uuid::Uuid;
    ///
    /// let doc = Document::new(Uuid::new_v4(), "My Document", "Hello, world!", DocumentFormat::Markdown);
    /// assert_eq!(doc.title(), "My Document");
    /// ```
    pub fn new(id: Uuid, title: &str, content: &str, format: DocumentFormat) -> Self {
        let now = SystemTime::now();
        
        Self {
            id,
            title: title.to_string(),
            content: content.to_string(),
            format,
            file_path: None,
            created_at: now,
            modified_at: now,
            has_unsaved_changes: true,
            metadata: DocumentMetadata::new(),
        }
    }

    /// Get the document ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the document title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the document title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        self.mark_modified();
    }

    /// Get the document content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the document content.
    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
        self.mark_modified();
    }

    /// Get the document format.
    pub fn format(&self) -> DocumentFormat {
        self.format
    }

    /// Get the file path.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Set the file path.
    pub fn set_file_path<P: AsRef<Path>>(&mut self, path: P) {
        self.file_path = Some(path.as_ref().to_path_buf());
    }

    /// Check if the document has unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    /// Get the creation time.
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    /// Get the last modification time.
    pub fn last_modified_time(&self) -> SystemTime {
        self.modified_at
    }

    /// Get the document metadata.
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    /// Get mutable document metadata.
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }

    /// Mark the document as modified.
    fn mark_modified(&mut self) {
        self.modified_at = SystemTime::now();
        self.has_unsaved_changes = true;
    }

    /// Mark the document as saved.
    pub(crate) fn mark_saved(&mut self) {
        self.has_unsaved_changes = false;
    }
}

/// Document format enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentFormat {
    /// Markdown format
    Markdown,
    /// Plain text format
    PlainText,
    /// Rich text format
    RichText,
    /// HTML format
    Html,
}

impl DocumentFormat {
    /// Get the file extension for this format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::document::DocumentFormat;
    ///
    /// assert_eq!(DocumentFormat::Markdown.extension(), "md");
    /// assert_eq!(DocumentFormat::PlainText.extension(), "txt");
    /// ```
    pub fn extension(&self) -> &'static str {
        match self {
            DocumentFormat::Markdown => "md",
            DocumentFormat::PlainText => "txt",
            DocumentFormat::RichText => "rtf",
            DocumentFormat::Html => "html",
        }
    }

    /// Determine format from file extension.
    ///
    /// # Arguments
    ///
    /// * `extension` - File extension (without dot)
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::document::DocumentFormat;
    ///
    /// assert_eq!(DocumentFormat::from_extension(Some("md")), DocumentFormat::Markdown);
    /// assert_eq!(DocumentFormat::from_extension(Some("txt")), DocumentFormat::PlainText);
    /// assert_eq!(DocumentFormat::from_extension(None), DocumentFormat::PlainText);
    /// ```
    pub fn from_extension(extension: Option<&str>) -> Self {
        match extension {
            Some("md") | Some("markdown") => DocumentFormat::Markdown,
            Some("html") | Some("htm") => DocumentFormat::Html,
            Some("rtf") => DocumentFormat::RichText,
            _ => DocumentFormat::PlainText,
        }
    }
}

/// Document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document tags
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Word count (cached)
    pub word_count: Option<usize>,
    /// Character count (cached)
    pub character_count: Option<usize>,
}

impl DocumentMetadata {
    /// Create new empty metadata.
    pub fn new() -> Self {
        Self {
            tags: Vec::new(),
            properties: HashMap::new(),
            word_count: None,
            character_count: None,
        }
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;

    /// Create a temporary directory for tests without relying on the `tempfile` crate.
    /// This helper is intentionally simple and avoids adding a test-only dependency.
    fn make_tempdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("cosmarium_test_{}", n));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn test_document_manager_creation() {
        let manager = DocumentManager::new();
        assert!(!manager.initialized);
        assert!(manager.documents.is_empty());
    }

    #[tokio::test]
    async fn test_document_manager_initialization() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = DocumentManager::new();
        
        assert!(manager.initialize(event_bus).await.is_ok());
        assert!(manager.initialized);
    }

    #[tokio::test]
    async fn test_document_creation() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = DocumentManager::new();
        manager.initialize(event_bus).await.unwrap();
        
        let doc_id = manager.create_document(
            "Test Document",
            "This is test content",
            DocumentFormat::Markdown
        ).await.unwrap();
        
        let document = manager.get_document(doc_id).unwrap();
        assert_eq!(document.title(), "Test Document");
        assert_eq!(document.content(), "This is test content");
        assert_eq!(document.format(), DocumentFormat::Markdown);
        assert!(document.has_unsaved_changes());
    }

    #[tokio::test]
    async fn test_document_modification() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = DocumentManager::new();
        manager.initialize(event_bus).await.unwrap();
        
        let doc_id = manager.create_document(
            "Test",
            "Original content",
            DocumentFormat::PlainText
        ).await.unwrap();
        
        let document = manager.get_document_mut(doc_id).unwrap();
        document.set_content("Modified content");
        
        assert_eq!(document.content(), "Modified content");
        assert!(document.has_unsaved_changes());
    }

    #[tokio::test]
    async fn test_document_formats() {
        assert_eq!(DocumentFormat::Markdown.extension(), "md");
        assert_eq!(DocumentFormat::PlainText.extension(), "txt");
        
        assert_eq!(DocumentFormat::from_extension(Some("md")), DocumentFormat::Markdown);
        assert_eq!(DocumentFormat::from_extension(Some("html")), DocumentFormat::Html);
        assert_eq!(DocumentFormat::from_extension(None), DocumentFormat::PlainText);
    }

    #[tokio::test]
    async fn test_document_metadata() {
        let mut metadata = DocumentMetadata::new();
        metadata.tags.push("fiction".to_string());
        metadata.properties.insert("genre".to_string(), "mystery".to_string());
        
        assert_eq!(metadata.tags.len(), 1);
        assert_eq!(metadata.properties.get("genre"), Some(&"mystery".to_string()));
    }

    #[tokio::test]
    async fn test_document_close() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = DocumentManager::new();
        manager.initialize(event_bus).await.unwrap();
        
        let doc_id = manager.create_document(
            "Test",
            "Content",
            DocumentFormat::PlainText
        ).await.unwrap();
        
        assert!(manager.get_document(doc_id).is_some());
        
        manager.close_document(doc_id, false).await.unwrap();
        assert!(manager.get_document(doc_id).is_none());
    }

    #[test]
    fn test_document_creation_direct() {
        let id = Uuid::new_v4();
        let doc = Document::new(id, "Test", "Content", DocumentFormat::Markdown);
        
        assert_eq!(doc.id(), id);
        assert_eq!(doc.title(), "Test");
        assert_eq!(doc.content(), "Content");
        assert_eq!(doc.format(), DocumentFormat::Markdown);
        assert!(doc.has_unsaved_changes());
    }
}