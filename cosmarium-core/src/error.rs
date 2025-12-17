//! # Error handling for Cosmarium Core
//!
//! This module provides a unified error handling system for the Cosmarium
//! creative writing software. It defines the main error types and result
//! types used throughout the application.
//!
//! The error system is designed to be extensible, allowing plugins to
//! define their own error types while still integrating with the core
//! error handling infrastructure.

use thiserror::Error;

/// Result type used throughout Cosmarium Core.
///
/// This is a type alias for `std::result::Result` with our custom [`Error`] type.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::{Result, Error};
///
/// fn example_function() -> Result<String> {
///     Ok("Success".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for Cosmarium Core.
///
/// This enum represents all possible errors that can occur within the core
/// system. It uses `thiserror` for automatic `std::error::Error` implementation
/// and provides structured error information.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration-related errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Plugin system errors
    #[error("Plugin error: {message}")]
    Plugin { message: String },

    /// Project management errors
    #[error("Project error: {message}")]
    Project { message: String },

    /// Document management errors
    #[error("Document error: {message}")]
    Document { message: String },

    /// Layout management errors
    #[error("Layout error: {message}")]
    Layout { message: String },

    /// Event system errors
    #[error("Event error: {message}")]
    Event { message: String },

    /// File I/O errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML serialization/deserialization errors
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// ZIP file handling errors
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// File watching errors
    #[error("File watcher error: {0}")]
    Notify(#[from] notify::Error),

    /// Generic error with custom message
    #[error("Error: {message}")]
    Generic { message: String },

    /// Validation errors
    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    /// Not found errors
    #[error("Not found: {resource}")]
    NotFound { resource: String },

    /// Already exists errors
    #[error("Already exists: {resource}")]
    AlreadyExists { resource: String },

    /// Permission denied errors
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },

    /// Timeout errors
    #[error("Operation timed out: {operation}")]
    Timeout { operation: String },

    /// Network-related errors
    #[error("Network error: {message}")]
    Network { message: String },

    /// Database errors
    #[error("Database error: {message}")]
    Database { message: String },
}

impl Error {
    /// Create a new configuration error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::config("Invalid configuration file format");
    /// ```
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a new plugin error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::plugin("Failed to load plugin: missing dependency");
    /// ```
    pub fn plugin<S: Into<String>>(message: S) -> Self {
        Self::Plugin {
            message: message.into(),
        }
    }

    /// Create a new project error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::project("Project file is corrupted");
    /// ```
    pub fn project<S: Into<String>>(message: S) -> Self {
        Self::Project {
            message: message.into(),
        }
    }

    /// Create a new document error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::document("Document format not supported");
    /// ```
    pub fn document<S: Into<String>>(message: S) -> Self {
        Self::Document {
            message: message.into(),
        }
    }

    /// Create a new layout error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::layout("Invalid panel configuration");
    /// ```
    pub fn layout<S: Into<String>>(message: S) -> Self {
        Self::Layout {
            message: message.into(),
        }
    }

    /// Create a new event error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::event("Event handler registration failed");
    /// ```
    pub fn event<S: Into<String>>(message: S) -> Self {
        Self::Event {
            message: message.into(),
        }
    }

    /// Create a new generic error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::generic("Something went wrong");
    /// ```
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Create a new validation error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::validation("email", "Invalid email format");
    /// ```
    pub fn validation<S: Into<String>>(field: S, message: S) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a new not found error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::not_found("Plugin 'markdown-editor'");
    /// ```
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create a new already exists error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::already_exists("Project file");
    /// ```
    pub fn already_exists<S: Into<String>>(resource: S) -> Self {
        Self::AlreadyExists {
            resource: resource.into(),
        }
    }

    /// Create a new permission denied error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::permission_denied("Write to configuration directory");
    /// ```
    pub fn permission_denied<S: Into<String>>(operation: S) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
        }
    }

    /// Create a new timeout error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::timeout("Plugin initialization");
    /// ```
    pub fn timeout<S: Into<String>>(operation: S) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// Create a new network error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::network("Connection refused");
    /// ```
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a new database error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::database("Connection pool exhausted");
    /// ```
    pub fn database<S: Into<String>>(message: S) -> Self {
        Self::Database {
            message: message.into(),
        }
    }

    /// Check if this error is a configuration error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::config("Invalid format");
    /// assert!(error.is_config());
    /// ```
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config { .. })
    }

    /// Check if this error is a plugin error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::plugin("Load failed");
    /// assert!(error.is_plugin());
    /// ```
    pub fn is_plugin(&self) -> bool {
        matches!(self, Self::Plugin { .. })
    }

    /// Check if this error is an I/O error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    /// use std::io;
    ///
    /// let error = Error::Io(io::Error::new(io::ErrorKind::NotFound, "File not found"));
    /// assert!(error.is_io());
    /// ```
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    /// Check if this error is a validation error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::validation("field", "message");
    /// assert!(error.is_validation());
    /// ```
    pub fn is_validation(&self) -> bool {
        matches!(self, Self::Validation { .. })
    }

    /// Get the error category as a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Error;
    ///
    /// let error = Error::config("Invalid format");
    /// assert_eq!(error.category(), "Config");
    /// ```
    pub fn category(&self) -> &'static str {
        match self {
            Self::Config { .. } => "Config",
            Self::Plugin { .. } => "Plugin",
            Self::Project { .. } => "Project",
            Self::Document { .. } => "Document",
            Self::Layout { .. } => "Layout",
            Self::Event { .. } => "Event",
            Self::Io(_) => "IO",
            Self::Json(_) => "JSON",
            Self::Toml(_) => "TOML",
            Self::Zip(_) => "ZIP",
            Self::Notify(_) => "FileWatcher",
            Self::Generic { .. } => "Generic",
            Self::Validation { .. } => "Validation",
            Self::NotFound { .. } => "NotFound",
            Self::AlreadyExists { .. } => "AlreadyExists",
            Self::PermissionDenied { .. } => "PermissionDenied",
            Self::Timeout { .. } => "Timeout",
            Self::Network { .. } => "Network",
            Self::Database { .. } => "Database",
        }
    }
}

/// Convenience macro for creating errors with context.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::{error, Error};
///
/// let err = error!("Failed to load {}: {}", "config.toml", "Permission denied");
/// ```
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::Error::generic(format!($($arg)*))
    };
}

/// Convenience macro for creating configuration errors.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::{config_error, Error};
///
/// let err = config_error!("Invalid value for {}: {}", "font_size", "not a number");
/// ```
#[macro_export]
macro_rules! config_error {
    ($($arg:tt)*) => {
        $crate::Error::config(format!($($arg)*))
    };
}

/// Convenience macro for creating plugin errors.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::{plugin_error, Error};
///
/// let err = plugin_error!("Plugin '{}' not found", "markdown-editor");
/// ```
#[macro_export]
macro_rules! plugin_error {
    ($($arg:tt)*) => {
        $crate::Error::plugin(format!($($arg)*))
    };
}

/// Convert from `anyhow::Error` to our custom error type.
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::generic(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_creation() {
        let error = Error::config("Test message");
        assert!(error.is_config());
        assert_eq!(error.category(), "Config");
        assert!(error.to_string().contains("Test message"));
    }

    #[test]
    fn test_plugin_error() {
        let error = Error::plugin("Plugin not found");
        assert!(error.is_plugin());
        assert_eq!(error.category(), "Plugin");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = Error::from(io_error);
        assert!(error.is_io());
        assert_eq!(error.category(), "IO");
    }

    #[test]
    fn test_validation_error() {
        let error = Error::validation("email", "Invalid format");
        assert!(error.is_validation());
        assert_eq!(error.category(), "Validation");
    }

    #[test]
    fn test_not_found_error() {
        let error = Error::not_found("Resource");
        assert_eq!(error.category(), "NotFound");
    }

    #[test]
    fn test_error_macros() {
        let error = error!("Test {}", "message");
        assert_eq!(error.category(), "Generic");

        let config_err = config_error!("Config {}", "error");
        assert!(config_err.is_config());

        let plugin_err = plugin_error!("Plugin {}", "error");
        assert!(plugin_err.is_plugin());
    }

    #[test]
    fn test_anyhow_conversion() {
        let anyhow_err = anyhow::anyhow!("Test error");
        let error = Error::from(anyhow_err);
        assert_eq!(error.category(), "Generic");
    }

    #[test]
    fn test_error_display() {
        let error = Error::config("Invalid configuration");
        let display = format!("{}", error);
        assert!(display.contains("Configuration error"));
        assert!(display.contains("Invalid configuration"));
    }

    #[test]
    fn test_error_debug() {
        let error = Error::config("Test");
        let debug = format!("{:?}", error);
        assert!(debug.contains("Config"));
        assert!(debug.contains("Test"));
    }
}
