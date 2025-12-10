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
use egui::{Ui, Color32, Vec2};
use serde::{Deserialize, Serialize};
use egui_dock::{DockArea, Style, TabViewer, DockState, NodeIndex, SurfaceIndex, Split, Node};

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
    /// Word wrap
    pub word_wrap: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Distraction free mode
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
            show_line_numbers: true,
            distraction_free: false,
        }
    }
}

/// Core editor logic separated from UI
struct EditorCore {
    content: String,
    config: EditorConfig,
    stats: stats::WritingStats,
    has_changes: bool,
    text_edit_id: Option<egui::Id>,
    editor_state: editor::MarkdownEditor,
    current_title: String,
    last_save: std::time::Instant,
}

impl EditorCore {
    fn new() -> Self {
        Self {
            content: String::new(),
            config: EditorConfig::default(),
            stats: stats::WritingStats::default(),
            has_changes: false,
            text_edit_id: None,
            editor_state: editor::MarkdownEditor::new(),
            current_title: "Untitled".to_string(),
            last_save: std::time::Instant::now(),
        }
    }

    /// Render the main editor UI
    fn render_editor(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {

        
        // Capture old content before editing
        let old_content = self.content.clone();

        // Calculate row height for scrolling
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        
        let mut scroll_area = egui::ScrollArea::vertical();
        
        // Handle goto line request by setting scroll offset
        if let Some(target_line) = ctx.get_shared_state::<usize>("markdown_editor_goto_line") {
            if target_line > 0 {
                let scroll_offset = (target_line as f32 - 1.0) * row_height;
                scroll_area = scroll_area.vertical_scroll_offset(scroll_offset);
                
                // Also move cursor to that line
                let char_idx = self.content.lines()
                    .take(target_line - 1)
                    .map(|l| l.chars().count() + 1)
                    .sum::<usize>();
                    
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), self.text_edit_id.unwrap_or(egui::Id::new("editor"))) {
                    let ccursor = egui::text::CCursor::new(char_idx);
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    egui::TextEdit::store_state(ui.ctx(), self.text_edit_id.unwrap_or(egui::Id::new("editor")), state);
                }
                
                // Clear request
                ctx.set_shared_state("markdown_editor_goto_line", 0usize);
            }
        }

        let output = scroll_area.show(ui, |ui| {
            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .lock_focus(true)
            )
        });
        
        let response = output.inner;
        
        // Request focus on first render or when clicked
        if response.clicked() || self.text_edit_id.is_none() {
            response.request_focus();
        }
        
        // Store the ID for later state retrieval
        self.text_edit_id = Some(response.id);

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
        
        // Publish stats to shared state for status bar
        ctx.set_shared_state("editor_word_count", self.stats.word_count());
        ctx.set_shared_state("editor_char_count", self.stats.char_count());
        ctx.set_shared_state("editor_para_count", self.stats.paragraph_count());

        // Publish cursor line
        if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
            if let Some(cursor) = state.cursor.char_range() {
                 let cursor_idx = cursor.primary.index;
                 // Calculate line number (1-based)
                 let line = self.content.chars().take(cursor_idx).filter(|&c| c == '\n').count() + 1;
                 ctx.set_shared_state("markdown_editor_cursor_line", line);
            }
        }
    }

    /// Render the statistics bar
    fn render_stats_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("Words: {}", self.stats.word_count()));
            ui.separator();
            ui.label(format!("Chars: {}", self.stats.char_count()));
            ui.separator();
            ui.label(format!("Paras: {}", self.stats.paragraph_count()));
        });
    }

    /// Update writing statistics based on current content
    fn update_stats(&mut self) {
        self.stats.update(&self.content);
    }

    /// Get dynamic title based on cursor position
    fn get_dynamic_title(&self, ctx: &egui::Context) -> String {
        if let Some(id) = self.text_edit_id {
            if let Some(state) = egui::TextEdit::load_state(ctx, id) {
                if let Some(cursor) = state.cursor.char_range() {
                    let cursor_idx = cursor.primary.index;
                    
                    // Convert char index to byte index to avoid panics with multi-byte chars
                    let byte_index = self.content.char_indices()
                        .map(|(i, _)| i)
                        .nth(cursor_idx)
                        .unwrap_or(self.content.len());
                        
                    // Find the nearest heading before cursor
                    let content_before = &self.content[..byte_index];
                    for line in content_before.lines().rev() {
                        let trimmed = line.trim();
                        if trimmed.starts_with('#') {
                            let title = trimmed.trim_start_matches('#').trim().to_string();
                            // tracing::debug!("Found title: {}", title);
                            return title;
                        }
                    }
                }
            }
        }
        
        // Default title if no heading found
        "Untitled".to_string()
    }
}

/// Actions that can be performed on the dock state
enum DockAction {
    SplitHorizontal(SurfaceIndex, NodeIndex),
    SplitVertical(SurfaceIndex, NodeIndex),
}

/// The main markdown editor plugin
pub struct MarkdownEditorPlugin {
    core: EditorCore,
    /// Docking tree for layout management
    tree: DockState<String>,
}

impl Default for MarkdownEditorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

struct EditorViewer<'a> {
    core: &'a mut EditorCore,
    ctx: &'a mut PluginContext,
    pending_action: &'a mut Option<DockAction>,
}

impl<'a> TabViewer for EditorViewer<'a> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        // Use dynamic title if available, otherwise tab name
        if tab == "Main View" {
            self.core.current_title.clone().into()
        } else {
            tab.as_str().into()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _tab: &mut Self::Tab) {
        self.core.render_editor(ui, self.ctx);
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, _tab: &mut Self::Tab, surface: SurfaceIndex, node: NodeIndex) {
        if ui.button("Split Horizontal").clicked() {
            *self.pending_action = Some(DockAction::SplitHorizontal(surface, node));
            ui.close_menu();
        }
        if ui.button("Split Vertical").clicked() {
            *self.pending_action = Some(DockAction::SplitVertical(surface, node));
            ui.close_menu();
        }

        // Hack to hide default items (Close, Eject) added by egui_dock
        let style = ui.style_mut();
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        style.visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
        style.visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
        style.spacing.item_spacing = egui::Vec2::ZERO;
        style.spacing.button_padding = egui::Vec2::ZERO;
    }
}

impl MarkdownEditorPlugin {
    /// Create a new markdown editor plugin instance.
    pub fn new() -> Self {
        let tree = DockState::new(vec!["Main View".to_string()]);
        
        Self {
            core: EditorCore::new(),
            tree,
        }
    }

    pub fn content(&self) -> &str {
        &self.core.content
    }

    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        self.core.content = content.into();
        self.core.has_changes = true;
        self.core.update_stats();
    }

    pub fn has_changes(&self) -> bool {
        self.core.has_changes
    }

    pub fn stats(&self) -> &stats::WritingStats {
        &self.core.stats
    }

    fn handle_auto_save(&mut self, ctx: &mut PluginContext) {
        if !self.core.has_changes {
            return;
        }

        let elapsed = self.core.last_save.elapsed();
        if elapsed.as_secs() >= self.core.config.auto_save_interval {
            if let Err(e) = self.auto_save(ctx) {
                tracing::error!("Auto-save failed: {}", e);
            }
        }
    }

    fn auto_save(&mut self, ctx: &mut PluginContext) -> Result<()> {
        ctx.set_shared_state("markdown_editor_content", self.core.content.clone());
        let event = Event::new(EventType::DocumentSaved, "Auto-saved document");
        ctx.emit_event(event);
        self.core.has_changes = false;
        self.core.last_save = std::time::Instant::now();
        tracing::info!("Document auto-saved");
        Ok(())
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
        if let Some(config) = ctx.get_config::<EditorConfig>("markdown_editor") {
            self.core.config = config;
        } else {
            ctx.set_config("markdown_editor", &self.core.config);
        }

        #[cfg(feature = "syntax-highlighting")]
        if self.core.config.syntax_highlighting {
            self.core.highlighter = Some(syntax::MarkdownHighlighter::new()?);
        }

        #[cfg(feature = "live-preview")]
        if self.core.config.live_preview {
            self.core.preview = Some(preview::PreviewRenderer::new());
        }

        if let Some(content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            self.core.content = content;
            self.core.update_stats();
        }

        tracing::info!("Markdown editor plugin initialized");
        Ok(())
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Editor
    }

    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        self.handle_auto_save(ctx);
        
        if let Some(action) = ctx.get_shared_state::<String>("markdown_editor_action") {
            match action.as_str() {
                "undo" => {
                    if let Some(previous_content) = self.core.editor_state.undo(self.core.content.clone()) {
                        self.core.content = previous_content;
                        self.core.has_changes = true;
                        self.core.update_stats();
                    }
                    ctx.set_shared_state("markdown_editor_action", "".to_string());
                }
                "redo" => {
                    if let Some(next_content) = self.core.editor_state.redo(self.core.content.clone()) {
                        self.core.content = next_content;
                        self.core.has_changes = true;
                        self.core.update_stats();
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
        if self.core.current_title.is_empty() {
            "Editor"
        } else {
            &self.core.current_title
        }
    }

    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        self.handle_auto_save(ctx);
        
        if let Some(action) = ctx.get_shared_state::<String>("markdown_editor_action") {
            match action.as_str() {
                "undo" => {
                    if let Some(previous_content) = self.core.editor_state.undo(self.core.content.clone()) {
                        self.core.content = previous_content;
                        self.core.has_changes = true;
                        self.core.update_stats();
                    }
                    ctx.set_shared_state("markdown_editor_action", "".to_string());
                }
                "redo" => {
                    if let Some(next_content) = self.core.editor_state.redo(self.core.content.clone()) {
                        self.core.content = next_content;
                        self.core.has_changes = true;
                        self.core.update_stats();
                    }
                    ctx.set_shared_state("markdown_editor_action", "".to_string());
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        // Update dynamic title for the active tab
        // We do this here so it's available for the next frame's panel_title call
        let new_title = self.core.get_dynamic_title(ui.ctx());
        if new_title != self.core.current_title {
            self.core.current_title = new_title;
        }
        
        // Update shared state for other plugins (like Outline)
        if self.core.has_changes {
            ctx.set_shared_state("markdown_editor_content", self.core.content.clone());
        }
        
        let mut pending_action = None;
        
        let mut viewer = EditorViewer {
            core: &mut self.core,
            ctx,
            pending_action: &mut pending_action,
        };
        
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(ui.style().as_ref()))
            .show(ui.ctx(), &mut viewer);
            
        // Handle any pending actions from context menus
        if let Some(action) = pending_action {
            let new_tab = format!("Editor {}", self.tree.iter_all_tabs().count() + 1);
            match action {
                DockAction::SplitHorizontal(surface, node) => {
                    self.tree.split((surface, node), Split::Right, 0.5, Node::leaf(new_tab));
                }
                DockAction::SplitVertical(surface, node) => {
                    self.tree.split((surface, node), Split::Below, 0.5, Node::leaf(new_tab));
                }
            }
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
        true
    }

    fn is_closable(&self) -> bool {
        false
    }

    fn context_menu_items(&self) -> Vec<cosmarium_plugin_api::PanelContextMenuItem> {
        use cosmarium_plugin_api::PanelContextMenuItem;
        
        vec![
            PanelContextMenuItem::new("save", "Save Document"),
            PanelContextMenuItem::new("export", "Export..."),
            PanelContextMenuItem::separator(),
            PanelContextMenuItem::new("word_wrap", if self.core.config.word_wrap { "Disable Word Wrap" } else { "Enable Word Wrap" }),
            PanelContextMenuItem::new("line_numbers", if self.core.config.show_line_numbers { "Hide Line Numbers" } else { "Show Line Numbers" }),
            PanelContextMenuItem::new("distraction_free", if self.core.config.distraction_free { "Exit Focus Mode" } else { "Enter Focus Mode" }),
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
                self.core.config.word_wrap = !self.core.config.word_wrap;
                ctx.set_config("markdown_editor", &self.core.config);
            }
            "line_numbers" => {
                self.core.config.show_line_numbers = !self.core.config.show_line_numbers;
                ctx.set_config("markdown_editor", &self.core.config);
            }
            "distraction_free" => {
                self.core.config.distraction_free = !self.core.config.distraction_free;
                ctx.set_config("markdown_editor", &self.core.config);
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
