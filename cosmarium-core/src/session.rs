//! # Session management for Cosmarium
//!
//! This module handles the persistence of user session data, such as the list of
//! recent projects and the last opened project. This data is stored separately
//! from the application configuration to keep user state distinct from settings.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// User session data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// List of recently opened projects, ordered by most recent first.
    pub recent_projects: Vec<PathBuf>,
    /// The last project that was opened.
    pub last_opened_project: Option<PathBuf>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            last_opened_project: None,
        }
    }
}

impl Session {
    /// Load session data from the default location.
    ///
    /// If the file doesn't exist or cannot be loaded, a default session is returned.
    pub fn load() -> Self {
        match Self::load_internal() {
            Ok(session) => session,
            Err(e) => {
                tracing::warn!("Failed to load session: {}. Using default.", e);
                Self::default()
            }
        }
    }

    fn load_internal() -> Result<Self> {
        let path = Self::session_file_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::config(format!("Failed to read session file: {}", e)))?;

        let session: Session = serde_json::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse session file: {}", e)))?;

        Ok(session)
    }

    /// Save session data to the default location.
    pub fn save(&self) -> Result<()> {
        let path = Self::session_file_path()?;
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::config(format!("Failed to create session directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::config(format!("Failed to serialize session: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| Error::config(format!("Failed to write session file: {}", e)))?;

        Ok(())
    }

    /// Add a project to the recent projects list.
    ///
    /// This moves the project to the top of the list if it already exists,
    /// and trims the list to the specified maximum size.
    pub fn add_recent_project(&mut self, path: PathBuf, max_count: usize) {
        // Remove existing entry if present
        self.recent_projects.retain(|p| p != &path);
        
        // Add to front
        self.recent_projects.insert(0, path.clone());
        
        // Update last opened
        self.last_opened_project = Some(path);
        
        // Trim list
        if self.recent_projects.len() > max_count {
            self.recent_projects.truncate(max_count);
        }
    }

    /// Get the path to the session file.
    fn session_file_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| Error::config("Could not determine data directory"))?
            .join("cosmarium");
        
        Ok(data_dir.join("session.json"))
    }
}
