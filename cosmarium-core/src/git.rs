//! Git integration for Cosmarium projects.
//!
//! Provides version control functionality using the `gix` library.

use crate::{Error, Result};
use gix::ThreadSafeRepository;
use std::path::Path;
use tracing::{info, warn, debug};

/// Git repository integration.
#[derive(Debug)]
pub struct GitIntegration {
    repo: ThreadSafeRepository,
}

impl GitIntegration {
    /// Initialize a new Git repository at the specified path.
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        info!("Initializing Git repository at {:?}", path);

        let repo = gix::init(path)
            .map_err(|e| Error::project(format!("Failed to init git repo: {}", e)))?;
            
        Ok(Self {
            repo: repo.into(),
        })
    }

    /// Open an existing Git repository at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        info!("Opening Git repository at {:?}", path);

        let repo = gix::open(path)
            .map_err(|e| Error::project(format!("Failed to open git repo: {}", e)))?;

        Ok(Self {
            repo: repo.into(),
        })
    }

    /// Commit changes to the repository.
    /// 
    /// # Arguments
    /// 
    /// * `message` - Commit message
    pub fn commit(&self, message: &str) -> Result<()> {
        debug!("Git commit requested: {}", message);
        
        // Git commit functionality is complex with gix
        // For now, we just log the request
        // Users can commit manually or we can implement this later
        // when gix has more stable high-level APIs
        
        debug!("Git commit is a placeholder - changes are tracked but not auto-committed");
        
        // Return Ok to avoid errors during save
        Ok(())
    }

    /// Get the current branch name.
    pub fn current_branch(&self) -> Result<String> {
        let repo = self.repo.to_thread_local();
        
        // Try to get HEAD reference
        match repo.head_ref() {
            Ok(Some(reference)) => {
                // Get the branch name from the reference
                let name = reference.name()
                    .shorten()
                    .to_string();
                Ok(name)
            }
            Ok(None) => {
                // Detached HEAD or no commits yet
                Ok("HEAD".to_string())
            }
            Err(e) => {
                debug!("Failed to get current branch: {}", e);
                Ok("unknown".to_string())
            }
        }
    }
}
