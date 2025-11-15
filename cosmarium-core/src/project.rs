//! # Project management system for Cosmarium Core
//!
//! This module provides project management functionality for the Cosmarium
//! creative writing software. It handles project creation, loading, saving,
//! and organization of documents and resources within writing projects.
//!
//! Projects in Cosmarium can be stored as either compressed files (.cosmarium)
//! or as directory structures, providing flexibility for different workflows
//! and collaboration needs.

use crate::{Error, Result, events::EventBus};
use cosmarium_plugin_api::{Event, EventType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Project management system for Cosmarium.
///
/// The [`ProjectManager`] handles all project-related operations including
/// creation, loading, saving, and organizing writing projects. It supports
/// both compressed and directory-based project formats.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::project::ProjectManager;
/// use cosmarium_core::events::EventBus;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// # tokio_test::block_on(async {
/// let event_bus = Arc::new(RwLock::new(EventBus::new()));
/// let mut manager = ProjectManager::new();
/// manager.initialize(event_bus).await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
pub struct ProjectManager {
    /// Currently active project
    active_project: Option<Project>,
    /// Recently opened projects
    recent_projects: Vec<PathBuf>,
    /// Event bus for system communication
    event_bus: Option<Arc<RwLock<EventBus>>>,
    /// Whether the manager is initialized
    initialized: bool,
    /// Maximum number of recent projects to track
    max_recent_projects: usize,
    /// Default project directory
    default_project_directory: PathBuf,
}

impl ProjectManager {
    /// Create a new project manager.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::project::ProjectManager;
    ///
    /// let manager = ProjectManager::new();
    /// ```
    pub fn new() -> Self {
        let default_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Cosmarium Projects");

        Self {
            active_project: None,
            recent_projects: Vec::new(),
            event_bus: None,
            initialized: false,
            max_recent_projects: 10,
            default_project_directory: default_dir,
        }
    }

    /// Initialize the project manager.
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
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn initialize(&mut self, event_bus: Arc<RwLock<EventBus>>) -> Result<()> {
        if self.initialized {
            warn!("Project manager is already initialized");
            return Ok(());
        }

        info!("Initializing project manager");
        self.event_bus = Some(event_bus);
        
        // Ensure default project directory exists
        if let Err(e) = tokio::fs::create_dir_all(&self.default_project_directory).await {
            warn!("Failed to create default project directory: {}", e);
        }

        // Load recent projects list
        self.load_recent_projects().await?;

        self.initialized = true;
        info!("Project manager initialized");
        Ok(())
    }

    /// Shutdown the project manager.
    ///
    /// This method saves the current project if needed and cleans up resources.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.shutdown().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        info!("Shutting down project manager");

        // Save current project if needed
        let need_save = if let Some(project) = &self.active_project {
            project.has_unsaved_changes()
        } else {
            false
        };

        if need_save {
            // Temporarily take ownership to avoid mutable aliasing during async save.
            if let Some(mut project) = self.active_project.take() {
                if let Err(e) = project.save().await {
                    error!("Failed to save project: {}", e);
                }
                // Restore ownership after saving.
                self.active_project = Some(project);
            }
        }

        // Save recent projects list
        if let Err(e) = self.save_recent_projects().await {
            error!("Failed to save recent projects: {}", e);
        }

        self.active_project = None;
        self.initialized = false;

        info!("Project manager shutdown completed");
        Ok(())
    }

    /// Create a new project.
    ///
    /// # Arguments
    ///
    /// * `name` - Project name
    /// * `path` - Project location
    /// * `template` - Project template to use
    ///
    /// # Errors
    ///
    /// Returns an error if project creation fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use std::path::Path;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// // let project_path = Path::new("./test_project");
    /// // manager.create_project("My Novel", project_path, "novel").await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn create_project<P: AsRef<Path>>(
        &mut self,
        name: &str,
        path: P,
        template: &str,
    ) -> Result<()> {
        if !self.initialized {
            return Err(Error::project("Project manager not initialized"));
        }

        let path = path.as_ref();
        
        // Check if path already exists
        if path.exists() {
            return Err(Error::project(format!("Project path already exists: {:?}", path)));
        }

        // Create project directory
        tokio::fs::create_dir_all(path).await
            .map_err(|e| Error::project(format!("Failed to create project directory: {}", e)))?;

        let project = Project::new(name, path, template)?;
        
        // Save project to close current one if any
        let need_save = if let Some(current_project) = &self.active_project {
            current_project.has_unsaved_changes()
        } else {
            false
        };

        if need_save {
            // Temporarily take ownership to avoid mutable aliasing during async save.
            if let Some(mut current_project) = self.active_project.take() {
                current_project.save().await?;
                // Restore ownership after saving.
                self.active_project = Some(current_project);
            }
        }

        // Set as active project
        self.active_project = Some(project);
        self.add_to_recent_projects(path.to_path_buf());

        // Emit project created event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::ProjectCreated, format!("Created project: {}", name));
            let _ = bus.emit(event).await;
        }

        info!("Created new project '{}' at {:?}", name, path);
        Ok(())
    }

    /// Open an existing project.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the project file or directory
    ///
    /// # Errors
    ///
    /// Returns an error if the project cannot be opened.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use std::path::Path;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let project_path = Path::new("./my_project");
    /// manager.open_project(project_path).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn open_project<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(Error::project(format!("Project path does not exist: {:?}", path)));
        }

        // Save current project if any
        let need_save = if let Some(current_project) = &self.active_project {
            current_project.has_unsaved_changes()
        } else {
            false
        };

        if need_save {
            // Temporarily take ownership to avoid mutable aliasing during async save.
            if let Some(mut current_project) = self.active_project.take() {
                current_project.save().await?;
                // Restore ownership after saving.
                self.active_project = Some(current_project);
            }
        }

        // Load project
        let project = Project::load(path).await?;
        let project_name = project.name().to_string();

        self.active_project = Some(project);
        self.add_to_recent_projects(path.to_path_buf());

        // Emit project opened event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::ProjectOpened, format!("Opened project: {}", project_name));
            let _ = bus.emit(event).await;
        }

        info!("Opened project '{}' from {:?}", project_name, path);
        Ok(())
    }

    /// Save the current project.
    ///
    /// # Errors
    ///
    /// Returns an error if the project cannot be saved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// // manager.save_project().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn save_project(&mut self) -> Result<()> {
        let name;
        let path_buf;
        {
            let project = self.active_project.as_mut()
                .ok_or_else(|| Error::project("No active project"))?;
            name = project.name().to_string();
            path_buf = project.path().to_path_buf();
        }

        // Write project file outside of any long-lived mutable borrow.
        tokio::fs::write(&path_buf, name.as_bytes()).await
            .map_err(|e| Error::project(format!("Failed to write project: {}", e)))?;

        // Emit project saved event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::ProjectSaved, format!("Saved project: {}", name));
            let _ = bus.emit(event).await;
        }

        Ok(())
    }

    /// Close the current project.
    ///
    /// # Arguments
    ///
    /// * `save_if_modified` - Whether to save the project if it has unsaved changes
    ///
    /// # Errors
    ///
    /// Returns an error if the project cannot be closed or saved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// // manager.close_project(true).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn close_project(&mut self, save_if_modified: bool) -> Result<()> {
        let project = self.active_project.as_ref()
            .ok_or_else(|| Error::project("No active project"))?;
        let project_name = project.name().to_string();

        if save_if_modified && project.has_unsaved_changes() {
            // Take ownership briefly to save without aliasing.
            if let Some(mut p) = self.active_project.take() {
                p.save().await?;
                // we do not restore because the project is being closed
            }
        }

        self.active_project = None;

        // Emit project closed event
        if let Some(ref event_bus) = self.event_bus {
            let bus = event_bus.write().await;
            let event = Event::new(EventType::ProjectClosed, format!("Closed project: {}", project_name));
            let _ = bus.emit(event).await;
        }

        info!("Closed project '{}'", project_name);
        Ok(())
    }

    /// Get the currently active project.
    ///
    /// # Returns
    ///
    /// Reference to the active project, or None if no project is open.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let project = manager.active_project();
    /// assert!(project.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn active_project(&self) -> Option<&Project> {
        self.active_project.as_ref()
    }

    /// Get the currently active project mutably.
    ///
    /// # Returns
    ///
    /// Mutable reference to the active project, or None if no project is open.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let project = manager.active_project_mut();
    /// assert!(project.is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn active_project_mut(&mut self) -> Option<&mut Project> {
        self.active_project.as_mut()
    }

    /// Get the list of recently opened projects.
    ///
    /// # Returns
    ///
    /// Vector of paths to recently opened projects.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    ///
    /// let recent = manager.recent_projects();
    /// assert!(recent.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub fn recent_projects(&self) -> &[PathBuf] {
        &self.recent_projects
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
    /// use cosmarium_core::{project::ProjectManager, events::EventBus};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// # tokio_test::block_on(async {
    /// let event_bus = Arc::new(RwLock::new(EventBus::new()));
    /// let mut manager = ProjectManager::new();
    /// manager.initialize(event_bus).await?;
    /// manager.update().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # });
    /// ```
    pub async fn update(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        // Update active project if any
        if let Some(ref mut project) = self.active_project {
            project.update().await?;
        }

        Ok(())
    }

    /// Add a project path to the recent projects list.
    fn add_to_recent_projects(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_projects.retain(|p| p != &path);
        
        // Add to front
        self.recent_projects.insert(0, path);
        
        // Limit to max size
        if self.recent_projects.len() > self.max_recent_projects {
            self.recent_projects.truncate(self.max_recent_projects);
        }
    }

    /// Internal method to save a project.
    async fn save_project_internal(&mut self, project: &mut Project) -> Result<()> {
        project.save().await?;
        debug!("Saved project '{}'", project.name());
        Ok(())
    }

    /// Load the recent projects list from storage.
    async fn load_recent_projects(&mut self) -> Result<()> {
        let recent_file = self.get_recent_projects_file();
        
        if recent_file.exists() {
            match tokio::fs::read_to_string(&recent_file).await {
                Ok(content) => {
                    if let Ok(projects) = serde_json::from_str::<Vec<PathBuf>>(&content) {
                        self.recent_projects = projects.into_iter()
                            .filter(|p| p.exists())
                            .take(self.max_recent_projects)
                            .collect();
                        debug!("Loaded {} recent projects", self.recent_projects.len());
                    }
                }
                Err(e) => {
                    warn!("Failed to load recent projects: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Save the recent projects list to storage.
    async fn save_recent_projects(&self) -> Result<()> {
        let recent_file = self.get_recent_projects_file();
        
        if let Some(parent) = recent_file.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| Error::project(format!("Failed to create config directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(&self.recent_projects)
            .map_err(|e| Error::project(format!("Failed to serialize recent projects: {}", e)))?;

        tokio::fs::write(&recent_file, content).await
            .map_err(|e| Error::project(format!("Failed to write recent projects: {}", e)))?;

        Ok(())
    }

    /// Get the path to the recent projects file.
    fn get_recent_projects_file(&self) -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cosmarium")
            .join("recent_projects.json")
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A writing project in Cosmarium.
///
/// Projects contain documents, resources, and metadata for organizing
/// writing work. They can be stored as directories or compressed files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project metadata
    metadata: ProjectMetadata,
    /// Project directory path
    path: PathBuf,
    /// Document references
    documents: Vec<Uuid>,
    /// Whether the project has unsaved changes
    has_unsaved_changes: bool,
    /// Project settings
    settings: ProjectSettings,
}

impl Project {
    /// Create a new project.
    ///
    /// # Arguments
    ///
    /// * `name` - Project name
    /// * `path` - Project directory path
    /// * `template` - Project template to use
    ///
    /// # Errors
    ///
    /// Returns an error if project creation fails.
    pub fn new<P: AsRef<Path>>(name: &str, path: P, template: &str) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let metadata = ProjectMetadata::new(name, template);
        
        let project = Self {
            metadata,
            path,
            documents: Vec::new(),
            has_unsaved_changes: true,
            settings: ProjectSettings::default(),
        };

        Ok(project)
    }

    /// Load a project from disk.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the project directory or file
    ///
    /// # Errors
    ///
    /// Returns an error if the project cannot be loaded.
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let project_file = path.join("project.json");
        
        if !project_file.exists() {
            return Err(Error::project("Project file not found"));
        }

        let content = tokio::fs::read_to_string(&project_file).await
            .map_err(|e| Error::project(format!("Failed to read project file: {}", e)))?;

        let mut project: Project = serde_json::from_str(&content)
            .map_err(|e| Error::project(format!("Failed to parse project file: {}", e)))?;

        project.path = path.to_path_buf();
        project.has_unsaved_changes = false;

        Ok(project)
    }

    /// Save the project to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the project cannot be saved.
    pub async fn save(&mut self) -> Result<()> {
        let project_file = self.path.join("project.json");
        
        // Update metadata
        self.metadata.last_modified = SystemTime::now();

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::project(format!("Failed to serialize project: {}", e)))?;

        tokio::fs::write(&project_file, content).await
            .map_err(|e| Error::project(format!("Failed to write project file: {}", e)))?;

        self.has_unsaved_changes = false;
        Ok(())
    }

    /// Get the project name.
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Get the project path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the project metadata.
    pub fn metadata(&self) -> &ProjectMetadata {
        &self.metadata
    }

    /// Get mutable project metadata.
    pub fn metadata_mut(&mut self) -> &mut ProjectMetadata {
        self.mark_modified();
        &mut self.metadata
    }

    /// Check if the project has unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    /// Get the project settings.
    pub fn settings(&self) -> &ProjectSettings {
        &self.settings
    }

    /// Get mutable project settings.
    pub fn settings_mut(&mut self) -> &mut ProjectSettings {
        self.mark_modified();
        &mut self.settings
    }

    /// Add a document to the project.
    pub fn add_document(&mut self, document_id: Uuid) {
        if !self.documents.contains(&document_id) {
            self.documents.push(document_id);
            self.mark_modified();
        }
    }

    /// Remove a document from the project.
    pub fn remove_document(&mut self, document_id: Uuid) {
        if let Some(pos) = self.documents.iter().position(|&id| id == document_id) {
            self.documents.remove(pos);
            self.mark_modified();
        }
    }

    /// Get the list of document IDs in the project.
    pub fn documents(&self) -> &[Uuid] {
        &self.documents
    }

    /// Update method for project maintenance.
    pub async fn update(&mut self) -> Result<()> {
        // Project-specific update logic would go here
        Ok(())
    }

    /// Mark the project as modified.
    fn mark_modified(&mut self) {
        self.has_unsaved_changes = true;
        self.metadata.last_modified = SystemTime::now();
    }
}

/// Project metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,
    /// Project description
    pub description: String,
    /// Project author
    pub author: String,
    /// Project version
    pub version: String,
    /// Creation time
    pub created: SystemTime,
    /// Last modification time
    pub last_modified: SystemTime,
    /// Project template used
    pub template: String,
    /// Project tags
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

impl ProjectMetadata {
    /// Create new project metadata.
    pub fn new(name: &str, template: &str) -> Self {
        let now = SystemTime::now();
        
        Self {
            name: name.to_string(),
            description: String::new(),
            author: String::new(),
            version: "1.0.0".to_string(),
            created: now,
            last_modified: now,
            template: template.to_string(),
            tags: Vec::new(),
            properties: HashMap::new(),
        }
    }
}

/// Project settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Whether to use compressed format
    pub use_compressed_format: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Backup settings
    pub backup_enabled: bool,
    /// Number of backups to keep
    pub backup_count: usize,
    /// Custom settings
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            use_compressed_format: false,
            auto_save_interval: 30,
            backup_enabled: true,
            backup_count: 5,
            custom: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;

    /// Create a temporary directory for tests without relying on the `tempfile` crate.
    /// If you prefer to use `tempfile`, enable a feature and adapt this helper.
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
    async fn test_project_manager_creation() {
        let manager = ProjectManager::new();
        assert!(!manager.initialized);
        assert!(manager.active_project.is_none());
        assert!(manager.recent_projects.is_empty());
    }

    #[tokio::test]
    async fn test_project_manager_initialization() {
        let event_bus = Arc::new(RwLock::new(EventBus::new()));
        let mut manager = ProjectManager::new();
        
        assert!(manager.initialize(event_bus).await.is_ok());
        assert!(manager.initialized);
    }

    #[tokio::test]
    async fn test_project_creation() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().join("test_project");
        
        let project = Project::new("Test Project", &project_path, "novel").unwrap();
        
        assert_eq!(project.name(), "Test Project");
        assert_eq!(project.path(), &project_path);
        assert!(project.has_unsaved_changes());
        assert_eq!(project.metadata().template, "novel");
    }

    #[tokio::test]
    async fn test_project_save_load() {
        let temp_dir = make_tempdir();
        let project_path = temp_dir.join("test_project");
        tokio::fs::create_dir_all(&project_path).await.unwrap();
        
        // Create and save project
        let mut project = Project::new("Test Project", &project_path, "novel").unwrap();
        project.save().await.unwrap();
        assert!(!project.has_unsaved_changes());
        
        // Load project
        let loaded_project = Project::load(&project_path).await.unwrap();
        assert_eq!(loaded_project.name(), "Test Project");
        assert_eq!(loaded_project.metadata().template, "novel");
        assert!(!loaded_project.has_unsaved_changes());
    }

    #[tokio::test]
    async fn test_project_document_management() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().join("test_project");
        
        let mut project = Project::new("Test Project", &project_path, "novel").unwrap();
        let doc_id = Uuid::new_v4();
        
        assert_eq!(project.documents().len(), 0);
        
        project.add_document(doc_id);
        assert_eq!(project.documents().len(), 1);
        assert!(project.documents().contains(&doc_id));
        assert!(project.has_unsaved_changes());
        
        project.remove_document(doc_id);
        assert_eq!(project.documents().len(), 0);
    }

    #[test]
    fn test_project_metadata() {
        let metadata = ProjectMetadata::new("Test Project", "novel");
        assert_eq!(metadata.name, "Test Project");
        assert_eq!(metadata.template, "novel");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.description.is_empty());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_project_settings_default() {
        let settings = ProjectSettings::default();
        assert!(!settings.use_compressed_format);
        assert_eq!(settings.auto_save_interval, 30);
        assert!(settings.backup_enabled);
        assert_eq!(settings.backup_count, 5);
        assert!(settings.custom.is_empty());
    }
}