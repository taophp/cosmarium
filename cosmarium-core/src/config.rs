//! # Configuration management for Cosmarium Core
//!
//! This module provides configuration management functionality for the Cosmarium
//! creative writing software. It handles loading, saving, and validating
//! application settings from various sources including files, environment
//! variables, and defaults.
//!
//! The configuration system is designed to be hierarchical, with settings
//! loaded in the following order of priority:
//! 1. Command line arguments (highest priority)
//! 2. Environment variables
//! 3. Configuration file
//! 4. Default values (lowest priority)

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Main configuration structure for Cosmarium.
///
/// This struct contains all configurable settings for the application,
/// organized into logical groups. It supports serialization/deserialization
/// for persistence and provides validation methods.
///
/// # Example
///
/// ```rust
/// use cosmarium_core::Config;
///
/// let config = Config::default();
/// assert_eq!(config.ui.theme, "dark");
/// assert_eq!(config.editor.font_size, 14.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Application-wide settings
    pub app: AppConfig,
    /// User interface settings
    pub ui: UiConfig,
    /// Editor settings
    pub editor: EditorConfig,
    /// Plugin settings
    pub plugins: PluginConfig,
    /// Project settings
    pub project: ProjectConfig,
    /// Export settings
    pub export: ExportConfig,
    /// Advanced/experimental settings
    pub advanced: AdvancedConfig,
}

/// Application-wide configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application language (ISO 639-1 code)
    pub language: String,
    /// Whether to check for updates on startup
    pub check_updates: bool,
    /// Whether to send anonymous usage statistics
    pub telemetry: bool,
    /// Auto-save interval in seconds (0 = disabled)
    pub auto_save_interval: u64,
    /// Maximum number of recent projects to remember
    pub max_recent_projects: usize,
    /// Whether to restore session on startup
    pub restore_session: bool,
}

/// User interface configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// UI theme name
    pub theme: String,
    /// Base font size
    pub font_size: f32,
    /// Font family for UI text
    pub font_family: String,
    /// Whether to use system font scaling
    pub system_font_scaling: bool,
    /// Window width on startup
    pub window_width: f32,
    /// Window height on startup
    pub window_height: f32,
    /// Whether to maximize window on startup
    pub maximize_on_startup: bool,
    /// Whether to show splash screen
    pub show_splash: bool,
    /// Panel animation duration in milliseconds
    pub animation_duration: u64,
    /// Whether to use smooth scrolling
    pub smooth_scrolling: bool,
}

/// Text editor configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Editor font family
    pub font_family: String,
    /// Editor font size
    pub font_size: f32,
    /// Line height multiplier
    pub line_height: f32,
    /// Tab size in spaces
    pub tab_size: usize,
    /// Whether to use soft tabs (spaces)
    pub use_soft_tabs: bool,
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Whether to highlight current line
    pub highlight_current_line: bool,
    /// Whether to enable word wrap
    pub word_wrap: bool,
    /// Word wrap column (0 = use window width)
    pub word_wrap_column: usize,
    /// Whether to show whitespace characters
    pub show_whitespace: bool,
    /// Whether to trim trailing whitespace on save
    pub trim_trailing_whitespace: bool,
    /// Auto-indent style
    pub auto_indent: String,
    /// Spell check language
    pub spell_check_language: String,
    /// Whether spell check is enabled
    pub spell_check_enabled: bool,
}

/// Plugin system configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Whether plugins are enabled
    pub enabled: bool,
    /// List of enabled plugin names
    pub enabled_plugins: Vec<String>,
    /// List of disabled plugin names
    pub disabled_plugins: Vec<String>,
    /// Plugin directories to search
    pub plugin_directories: Vec<PathBuf>,
    /// Whether to auto-load plugins on startup
    pub auto_load: bool,
    /// Whether to check plugin signatures
    pub check_signatures: bool,
    /// Plugin-specific settings
    pub plugin_settings: HashMap<String, serde_json::Value>,
}

/// Project management configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Default project directory
    pub default_directory: PathBuf,
    /// Whether to use compressed project files
    pub use_compressed_format: bool,
    /// Project backup settings
    pub backup_enabled: bool,
    /// Number of backups to keep
    pub backup_count: usize,
    /// Backup interval in minutes
    pub backup_interval: u64,
    /// Whether to create project templates
    pub enable_templates: bool,
    /// Default project template
    pub default_template: String,
}

/// Export configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Default export directory
    pub default_directory: PathBuf,
    /// Default export format
    pub default_format: String,
    /// PDF export settings
    pub pdf: PdfExportConfig,
    /// HTML export settings
    pub html: HtmlExportConfig,
    /// Word export settings
    pub word: WordExportConfig,
}

/// PDF export specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExportConfig {
    /// Paper size (A4, Letter, etc.)
    pub paper_size: String,
    /// Margins in millimeters
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    /// Font settings
    pub font_family: String,
    pub font_size: f32,
    /// Whether to include table of contents
    pub include_toc: bool,
    /// Whether to include page numbers
    pub include_page_numbers: bool,
}

/// HTML export specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlExportConfig {
    /// CSS theme for HTML export
    pub theme: String,
    /// Whether to include custom CSS
    pub include_custom_css: bool,
    /// Custom CSS content
    pub custom_css: String,
    /// Whether to export as single file
    pub single_file: bool,
    /// Whether to include table of contents
    pub include_toc: bool,
}

/// Word export specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordExportConfig {
    /// Document template to use
    pub template: String,
    /// Whether to preserve formatting
    pub preserve_formatting: bool,
    /// Whether to include comments
    pub include_comments: bool,
    /// Whether to track changes
    pub track_changes: bool,
}

/// Advanced/experimental configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Enable debug mode
    pub debug_mode: bool,
    /// Log level (error, warn, info, debug, trace)
    pub log_level: String,
    /// Log to file
    pub log_to_file: bool,
    /// Performance profiling enabled
    pub profiling_enabled: bool,
    /// Memory limit in MB (0 = unlimited)
    pub memory_limit: usize,
    /// Network timeout in seconds
    pub network_timeout: u64,
    /// Experimental features enabled
    pub experimental_features: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            ui: UiConfig::default(),
            editor: EditorConfig::default(),
            plugins: PluginConfig::default(),
            project: ProjectConfig::default(),
            export: ExportConfig::default(),
            advanced: AdvancedConfig::default(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            check_updates: true,
            telemetry: false,
            auto_save_interval: 30,
            max_recent_projects: 10,
            restore_session: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 12.0,
            font_family: "Inter".to_string(),
            system_font_scaling: true,
            window_width: 1200.0,
            window_height: 800.0,
            maximize_on_startup: false,
            show_splash: true,
            animation_duration: 200,
            smooth_scrolling: true,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.5,
            tab_size: 4,
            use_soft_tabs: true,
            show_line_numbers: false,
            highlight_current_line: true,
            word_wrap: true,
            word_wrap_column: 80,
            show_whitespace: false,
            trim_trailing_whitespace: true,
            auto_indent: "smart".to_string(),
            spell_check_language: "en_US".to_string(),
            spell_check_enabled: true,
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enabled_plugins: vec![
                "markdown-editor".to_string(),
                "notes-panel".to_string(),
                "export".to_string(),
            ],
            disabled_plugins: Vec::new(),
            plugin_directories: vec![
                PathBuf::from("plugins"),
                dirs::data_dir()
                    .unwrap_or_default()
                    .join("cosmarium")
                    .join("plugins"),
            ],
            auto_load: true,
            check_signatures: false,
            plugin_settings: HashMap::new(),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            default_directory: dirs::document_dir()
                .unwrap_or_default()
                .join("Cosmarium Projects"),
            use_compressed_format: true,
            backup_enabled: true,
            backup_count: 5,
            backup_interval: 10,
            enable_templates: true,
            default_template: "novel".to_string(),
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            default_directory: dirs::document_dir()
                .unwrap_or_default()
                .join("Cosmarium Exports"),
            default_format: "pdf".to_string(),
            pdf: PdfExportConfig::default(),
            html: HtmlExportConfig::default(),
            word: WordExportConfig::default(),
        }
    }
}

impl Default for PdfExportConfig {
    fn default() -> Self {
        Self {
            paper_size: "A4".to_string(),
            margin_top: 25.0,
            margin_bottom: 25.0,
            margin_left: 25.0,
            margin_right: 25.0,
            font_family: "Liberation Serif".to_string(),
            font_size: 11.0,
            include_toc: true,
            include_page_numbers: true,
        }
    }
}

impl Default for HtmlExportConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            include_custom_css: false,
            custom_css: String::new(),
            single_file: true,
            include_toc: true,
        }
    }
}

impl Default for WordExportConfig {
    fn default() -> Self {
        Self {
            template: "default".to_string(),
            preserve_formatting: true,
            include_comments: false,
            track_changes: false,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            debug_mode: false,
            log_level: "info".to_string(),
            log_to_file: false,
            profiling_enabled: false,
            memory_limit: 0,
            network_timeout: 30,
            experimental_features: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from the default location or create default config.
    ///
    /// This method attempts to load configuration from the standard config file
    /// location. If the file doesn't exist or can't be loaded, it returns the
    /// default configuration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Config;
    ///
    /// let config = Config::load_or_default().unwrap();
    /// ```
    pub fn load_or_default() -> Result<Self> {
        match Self::load() {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = Self::default();
                // Try to save default config
                let _ = config.save();
                Ok(config)
            }
        }
    }

    /// Load configuration from the default config file.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cosmarium_core::Config;
    ///
    /// let config = Config::load()?;
    /// # Ok::<(), cosmarium_core::Error>(())
    /// ```
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path()?;
        Self::load_from_file(&config_path)
    }

    /// Load configuration from a specific file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cosmarium_core::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::load_from_file(Path::new("my_config.toml"))?;
    /// # Ok::<(), cosmarium_core::Error>(())
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse config file: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to the default config file.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cosmarium_core::Config;
    ///
    /// let config = Config::default();
    /// config.save()?;
    /// # Ok::<(), cosmarium_core::Error>(())
    /// ```
    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_config_path()?;
        self.save_to_file(&config_path)
    }

    /// Save configuration to a specific file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to save the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cosmarium_core::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::default();
    /// config.save_to_file(Path::new("my_config.toml"))?;
    /// # Ok::<(), cosmarium_core::Error>(())
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.validate()?;

        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::config(format!("Failed to serialize config: {}", e)))?;

        // Ensure directory exists
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::config(format!("Failed to create config directory: {}", e)))?;
        }

        std::fs::write(path, content)
            .map_err(|e| Error::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate the configuration values.
    ///
    /// This method checks that all configuration values are within acceptable
    /// ranges and combinations.
    ///
    /// # Errors
    ///
    /// Returns a validation error if any configuration value is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Validate UI settings
        if self.ui.font_size <= 0.0 || self.ui.font_size > 72.0 {
            return Err(Error::validation(
                "ui.font_size",
                "Font size must be between 1 and 72",
            ));
        }

        if self.ui.window_width < 400.0 || self.ui.window_height < 300.0 {
            return Err(Error::validation(
                "ui.window_size",
                "Window size must be at least 400x300",
            ));
        }

        // Validate editor settings
        if self.editor.font_size <= 0.0 || self.editor.font_size > 72.0 {
            return Err(Error::validation(
                "editor.font_size",
                "Editor font size must be between 1 and 72",
            ));
        }

        if self.editor.tab_size == 0 || self.editor.tab_size > 16 {
            return Err(Error::validation(
                "editor.tab_size",
                "Tab size must be between 1 and 16",
            ));
        }

        if self.editor.line_height < 0.8 || self.editor.line_height > 3.0 {
            return Err(Error::validation(
                "editor.line_height",
                "Line height must be between 0.8 and 3.0",
            ));
        }

        // Validate app settings
        if self.app.max_recent_projects > 50 {
            return Err(Error::validation(
                "app.max_recent_projects",
                "Maximum recent projects cannot exceed 50",
            ));
        }

        // Validate project settings
        if self.project.backup_count > 20 {
            return Err(Error::validation(
                "project.backup_count",
                "Backup count cannot exceed 20",
            ));
        }

        // Validate advanced settings
        if !["error", "warn", "info", "debug", "trace"].contains(&self.advanced.log_level.as_str())
        {
            return Err(Error::validation(
                "advanced.log_level",
                "Log level must be one of: error, warn, info, debug, trace",
            ));
        }

        Ok(())
    }

    /// Get the default configuration file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the config directory cannot be determined.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Config;
    ///
    /// let path = Config::default_config_path()?;
    /// # Ok::<(), cosmarium_core::Error>(())
    /// ```
    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::config("Could not determine config directory"))?
            .join("cosmarium");

        Ok(config_dir.join("config.toml"))
    }

    /// Get the configuration directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the config directory cannot be determined.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Config;
    ///
    /// let dir = Config::config_dir()?;
    /// # Ok::<(), cosmarium_core::Error>(())
    /// ```
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join("cosmarium"))
            .ok_or_else(|| Error::config("Could not determine config directory"))
    }

    /// Reset configuration to defaults.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Config;
    ///
    /// let mut config = Config::default();
    /// config.ui.font_size = 20.0;
    /// config.reset_to_defaults();
    /// assert_eq!(config.ui.font_size, 12.0);
    /// ```
    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }

    /// Merge configuration from another source.
    ///
    /// This method allows partial configuration updates while preserving
    /// existing values for fields not present in the source.
    ///
    /// # Arguments
    ///
    /// * `other` - Configuration to merge from
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_core::Config;
    ///
    /// let mut config = Config::default();
    /// let mut other = Config::default();
    /// other.ui.font_size = 16.0;
    ///
    /// config.merge_from(&other);
    /// assert_eq!(config.ui.font_size, 16.0);
    /// ```
    pub fn merge_from(&mut self, other: &Self) {
        // This is a simplified merge - in a real implementation,
        // you might want more sophisticated merging logic
        *self = other.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.ui.theme, "dark");
        assert_eq!(config.editor.font_size, 14.0);
        assert_eq!(config.app.language, "en");
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        // Test invalid font size
        config.ui.font_size = 0.0;
        assert!(config.validate().is_err());

        // Reset and test invalid tab size
        config = Config::default();
        config.editor.tab_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.ui.theme, deserialized.ui.theme);
        assert_eq!(config.editor.font_size, deserialized.editor.font_size);
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let config = Config::default();

        // Test save
        assert!(config.save_to_file(&config_path).is_ok());
        assert!(config_path.exists());

        // Test load
        let loaded_config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.ui.theme, loaded_config.ui.theme);
        assert_eq!(config.editor.font_size, loaded_config.editor.font_size);
    }

    #[test]
    fn test_config_validation_errors() {
        let mut config = Config::default();

        // Test font size validation
        config.ui.font_size = 100.0;
        assert!(config.validate().is_err());

        // Test window size validation
        config = Config::default();
        config.ui.window_width = 200.0;
        assert!(config.validate().is_err());

        // Test log level validation
        config = Config::default();
        config.advanced.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config::default();
        let mut config2 = Config::default();

        config2.ui.font_size = 20.0;
        config2.editor.tab_size = 8;

        config1.merge_from(&config2);
        assert_eq!(config1.ui.font_size, 20.0);
        assert_eq!(config1.editor.tab_size, 8);
    }

    #[test]
    fn test_reset_to_defaults() {
        let mut config = Config::default();
        config.ui.font_size = 20.0;
        config.editor.tab_size = 8;

        config.reset_to_defaults();
        assert_eq!(config.ui.font_size, 12.0);
        assert_eq!(config.editor.tab_size, 4);
    }
}
