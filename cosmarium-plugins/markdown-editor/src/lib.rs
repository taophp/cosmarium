//! # Cosmarium Markdown Editor Plugin
//!
//! This plugin provides a rich markdown editor for the Cosmarium creative writing software.
//! It includes syntax highlighting, live preview, and writing assistance features specifically
//! designed for creative writers.
//!
//! ## Features
//!
//! - Immersive markdown editing with syntax highlighting
//! - Optional live preview panel
//! - Word count and writing statistics
//! - Distraction-free writing mode
//! - Auto-save functionality
//! - Custom shortcuts for writers
//!
//! ## Example
//!
//! ```rust
//! use cosmarium_markdown_editor::MarkdownEditorPlugin;
//! use cosmarium_plugin_api::{Plugin, PluginInfo};
//!
//! let mut editor = MarkdownEditorPlugin::new();
//! let info = editor.info();
//! assert_eq!(info.name, "markdown-editor");
//! ```

pub mod editor;
pub mod preview;
pub mod syntax;
pub mod stats;

use cosmarium_plugin_api::{
    Plugin, PluginInfo, PluginType, PluginContext, PanelPlugin, 
    Event, EventType, Result
};
use egui::Ui;
use serde::{Deserialize, Serialize};


/// Configuration for the markdown editor plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Enable live preview
    pub live_preview: bool,
    /// Font size for the editor
    pub font_size: f32,
    /// Tab size in spaces
    pub tab_size: usize,
    /// Word wrap enabled
    pub word_wrap: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Distraction-free mode
    pub distraction_free: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            syntax_highlighting: true,
            live_preview: false,
            font_size: 14.0,
            tab_size: 4,
            word_wrap: true,
            auto_save_interval: 30,
            show_line_numbers: false,
            distraction_free: false,
        }
    }
}

/// The main markdown editor plugin
pub struct MarkdownEditorPlugin {
    /// Current document content
    content: String,
    /// Plugin configuration
    config: EditorConfig,
    /// Whether the document has unsaved changes
    has_changes: bool,
    /// Last auto-save timestamp
    last_save: std::time::Instant,
    /// Word count and statistics
    stats: stats::WritingStats,
    /// Syntax highlighter
    #[cfg(feature = "syntax-highlighting")]
    highlighter: Option<syntax::MarkdownHighlighter>,
    /// Preview renderer
    #[cfg(feature = "live-preview")]
    preview: Option<preview::PreviewRenderer>,
    /// Editor state for history management
    editor_state: editor::MarkdownEditor,
}

impl Default for MarkdownEditorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownEditorPlugin {
    /// Create a new markdown editor plugin instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::MarkdownEditorPlugin;
    ///
    /// let editor = MarkdownEditorPlugin::new();
    /// ```
    pub fn new() -> Self {
        Self {
            content: String::new(),
            config: EditorConfig::default(),
            has_changes: false,
            last_save: std::time::Instant::now(),
            stats: stats::WritingStats::new(),
            #[cfg(feature = "syntax-highlighting")]
            highlighter: None,
            #[cfg(feature = "live-preview")]
            preview: None,
            editor_state: editor::MarkdownEditor::new(),
        }
    }

    /// Get the current document content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::MarkdownEditorPlugin;
    ///
    /// let editor = MarkdownEditorPlugin::new();
    /// let content = editor.content();
    /// assert!(content.is_empty());
    /// ```
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the document content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::MarkdownEditorPlugin;
    ///
    /// let mut editor = MarkdownEditorPlugin::new();
    /// editor.set_content("# Hello World\n\nThis is a test document.");
    /// assert_eq!(editor.content().lines().next(), Some("# Hello World"));
    /// ```
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        self.content = content.into();
        self.has_changes = true;
        self.update_stats();
    }

    /// Check if the document has unsaved changes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::MarkdownEditorPlugin;
    ///
    /// let mut editor = MarkdownEditorPlugin::new();
    /// assert!(!editor.has_changes());
    /// editor.set_content("Some content");
    /// assert!(editor.has_changes());
    /// ```
    pub fn has_changes(&self) -> bool {
        self.has_changes
    }

    /// Get the current writing statistics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::MarkdownEditorPlugin;
    ///
    /// let mut editor = MarkdownEditorPlugin::new();
    /// editor.set_content("Hello world! This is a test.");
    /// let stats = editor.stats();
    /// assert!(stats.word_count() > 0);
    /// ```
    pub fn stats(&self) -> &stats::WritingStats {
        &self.stats
    }

    /// Update writing statistics based on current content
    fn update_stats(&mut self) {
        self.stats.update(&self.content);
    }

    /// Handle auto-save if needed
    fn handle_auto_save(&mut self, ctx: &mut PluginContext) {
        if !self.has_changes {
            return;
        }

        let elapsed = self.last_save.elapsed();
        if elapsed.as_secs() >= self.config.auto_save_interval {
            if let Err(e) = self.auto_save(ctx) {
                tracing::error!("Auto-save failed: {}", e);
            }
        }
    }

    /// Perform auto-save
    fn auto_save(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Save content to shared state
        ctx.set_shared_state("markdown_editor_content", self.content.clone());
        
        // Emit document saved event
        let event = Event::new(EventType::DocumentSaved, "Auto-saved document");
        ctx.emit_event(event);
        
        self.has_changes = false;
        self.last_save = std::time::Instant::now();
        
        tracing::info!("Document auto-saved");
        Ok(())
    }

    /// Render the main editor UI
    fn render_editor(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        ui.horizontal(|ui| {
            ui.label("📝");
            ui.heading("Markdown Editor");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Word count display
                ui.label(format!("Words: {}", self.stats.word_count()));
                ui.separator();
                
                // Save indicator
                if self.has_changes {
                    ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "●");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "●");
                }
            });
        });
        
        ui.separator();
        
        // Capture old content before editing
        let old_content = self.content.clone();

        // Main text editor
        let response = ui.add_sized(
            ui.available_size(),
            egui::TextEdit::multiline(&mut self.content)
                .font(egui::TextStyle::Monospace)
                .desired_width(ui.available_width())
                .desired_rows(25)
        );
        
        // Handle content changes
        if response.changed() {
            self.has_changes = true;
            self.update_stats();
            
            // Emit document changed event
            let event = Event::new(EventType::DocumentChanged, "Document content modified");
            ctx.emit_event(event);

            // Push OLD content to history
            self.editor_state.add_to_history(old_content);
        }
        
        // Show statistics panel at the bottom
        ui.separator();
        self.render_stats_bar(ui);
    }

    /// Render the statistics bar
    fn render_stats_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Words: {}", self.stats.word_count()));
            ui.separator();
            ui.label(format!("Characters: {}", self.stats.char_count()));
            ui.separator();
            ui.label(format!("Paragraphs: {}", self.stats.paragraph_count()));
            
            if self.config.distraction_free {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("🎯 Focus Mode");
                });
            }
        });
    }
}

impl Plugin for MarkdownEditorPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new(
            "markdown-editor",
            "0.1.0",
            "Rich markdown editor for creative writing",
            "Cosmarium Team"
        )
        .with_dependency("cosmarium-core")
        .with_min_core_version("0.1.0")
    }

    fn initialize(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Load configuration from context
        if let Some(config) = ctx.get_config::<EditorConfig>("markdown_editor") {
            self.config = config;
        } else {
            // Save default config
            ctx.set_config("markdown_editor", &self.config);
        }

        // Initialize syntax highlighter if enabled
        #[cfg(feature = "syntax-highlighting")]
        if self.config.syntax_highlighting {
            self.highlighter = Some(syntax::MarkdownHighlighter::new()?);
        }

        // Initialize preview renderer if enabled
        #[cfg(feature = "live-preview")]
        if self.config.live_preview {
            self.preview = Some(preview::PreviewRenderer::new());
        }

        // Load existing content if available
        if let Some(content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            self.content = content;
            self.update_stats();
        }

        tracing::info!("Markdown editor plugin initialized");
        Ok(())
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Editor
    }

    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        self.handle_auto_save(ctx);
        
        // Check for undo/redo commands from shared state
        if let Some(action) = ctx.get_shared_state::<String>("markdown_editor_action") {
            match action.as_str() {
                "undo" => {
                    if let Some(previous_content) = self.editor_state.undo(self.content.clone()) {
                        self.content = previous_content;
                        self.has_changes = true;
                        self.update_stats();
                    }
                    // Clear the action so we don't repeat it
                    ctx.set_shared_state("markdown_editor_action", "".to_string());
                }
                "redo" => {
                    if let Some(next_content) = self.editor_state.redo(self.content.clone()) {
                        self.content = next_content;
                        self.has_changes = true;
                        self.update_stats();
                    }
                    ctx.set_shared_state("markdown_editor_action", "".to_string());
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}

impl PanelPlugin for MarkdownEditorPlugin {
    fn panel_title(&self) -> &str {
        "Markdown Editor"
    }

    fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        if self.config.distraction_free {
            // In distraction-free mode, show only the editor
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width())
            );
        } else {
            self.render_editor(ui, ctx);
        }
    }

    fn default_position(&self) -> cosmarium_plugin_api::PanelPosition {
        cosmarium_plugin_api::PanelPosition::Center
    }

    fn default_size(&self) -> cosmarium_plugin_api::PanelSize {
        cosmarium_plugin_api::PanelSize::Flexible {
            min_width: 400.0,
            min_height: 300.0,
            max_width: None,
            max_height: None,
        }
    }

    fn default_open(&self) -> bool {
        true // Main editor should be open by default
    }

    fn is_closable(&self) -> bool {
        false // Core editor should not be closable
    }

    fn context_menu_items(&self) -> Vec<cosmarium_plugin_api::PanelContextMenuItem> {
        use cosmarium_plugin_api::PanelContextMenuItem;
        
        vec![
            PanelContextMenuItem::new("save", "Save Document"),
            PanelContextMenuItem::new("export", "Export..."),
            PanelContextMenuItem::separator(),
            PanelContextMenuItem::new("word_wrap", if self.config.word_wrap { "Disable Word Wrap" } else { "Enable Word Wrap" }),
            PanelContextMenuItem::new("line_numbers", if self.config.show_line_numbers { "Hide Line Numbers" } else { "Show Line Numbers" }),
            PanelContextMenuItem::new("distraction_free", if self.config.distraction_free { "Exit Focus Mode" } else { "Enter Focus Mode" }),
            PanelContextMenuItem::separator(),
            PanelContextMenuItem::new("settings", "Editor Settings"),
        ]
    }

    fn handle_context_menu(&mut self, item_id: &str, ctx: &mut PluginContext) -> Result<()> {
        match item_id {
            "save" => {
                self.auto_save(ctx)?;
            }
            "word_wrap" => {
                self.config.word_wrap = !self.config.word_wrap;
                ctx.set_config("markdown_editor", &self.config);
            }
            "line_numbers" => {
                self.config.show_line_numbers = !self.config.show_line_numbers;
                ctx.set_config("markdown_editor", &self.config);
            }
            "distraction_free" => {
                self.config.distraction_free = !self.config.distraction_free;
                ctx.set_config("markdown_editor", &self.config);
            }
            _ => {
                tracing::warn!("Unhandled context menu item: {}", item_id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmarium_plugin_api::PluginContext;

    #[test]
    fn test_plugin_creation() {
        let editor = MarkdownEditorPlugin::new();
        assert!(editor.content().is_empty());
        assert!(!editor.has_changes());
    }

    #[test]
    fn test_content_modification() {
        let mut editor = MarkdownEditorPlugin::new();
        editor.set_content("# Test Document\n\nThis is a test.");
        
        assert!(editor.has_changes());
        assert_eq!(editor.content().lines().next(), Some("# Test Document"));
        assert!(editor.stats().word_count() > 0);
    }

    #[test]
    fn test_plugin_info() {
        let editor = MarkdownEditorPlugin::new();
        let info = editor.info();
        
        assert_eq!(info.name, "markdown-editor");
        assert_eq!(info.version, "0.1.0");
        assert!(info.dependencies.contains(&"cosmarium-core".to_string()));
    }

    #[tokio::test]
    async fn test_plugin_initialization() {
        let mut editor = MarkdownEditorPlugin::new();
        let mut ctx = PluginContext::new();
        
        assert!(editor.initialize(&mut ctx).is_ok());
        
        // Test that default config was saved
        let saved_config: Option<EditorConfig> = ctx.get_config("markdown_editor");
        assert!(saved_config.is_some());
    }

    #[test]
    fn test_auto_save_logic() {
        let mut editor = MarkdownEditorPlugin::new();
        let mut ctx = PluginContext::new();
        
        // Set content and trigger auto-save
        editor.set_content("Test content");
        assert!(editor.has_changes());
        
        assert!(editor.auto_save(&mut ctx).is_ok());
        assert!(!editor.has_changes());
        
        // Verify content was saved to shared state
        let saved_content: Option<String> = ctx.get_shared_state("markdown_editor_content");
        assert_eq!(saved_content, Some("Test content".to_string()));
    }
}
