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
use egui::text_edit::TextEditState;
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
    last_cursor_char_idx: Option<usize>,
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
            current_title: "Editor".to_string(),
            last_cursor_char_idx: None,
            last_save: std::time::Instant::now(),
        }
    }

    /// Render the main editor UI
    fn render_editor(&mut self, ui: &mut Ui, ctx: &mut PluginContext, tab_id: &str) {

        
        // Capture old content before editing
        let old_content = self.content.clone();

        // Calculate row height for scrolling
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        
        let mut scroll_area = egui::ScrollArea::vertical();
        
        // Initialize focus request flag
        let mut request_focus = self.text_edit_id.is_none();
        
        // Handle goto line request by setting scroll offset
        if let Some(target_line) = ctx.get_shared_state::<usize>("markdown_editor_goto_line") {
            if target_line > 0 {
                // Check if we are the target tab (last active tab)
                let last_active = ctx.get_shared_state::<String>("markdown_editor_last_active_tab");
                // If no last active tab is recorded, the first one to run will take it (fallback)
                let is_target = last_active.as_deref() == Some(tab_id) || last_active.is_none();
                
                if is_target {
                    let scroll_offset = (target_line as f32 - 1.0) * row_height;
                    scroll_area = scroll_area.vertical_scroll_offset(scroll_offset);
                    
                    // Move cursor to the END of that line
                    let lines: Vec<&str> = self.content.lines().collect();
                    if target_line <= lines.len() {
                        let pre_chars: usize = lines.iter().take(target_line - 1).map(|l| l.chars().count() + 1).sum();
                        let line_len = lines[target_line - 1].chars().count();
                        let char_idx = pre_chars + line_len;
                        
                        let edit_id = egui::Id::new("markdown_editor_textedit").with(tab_id);
                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), edit_id) {
                            let ccursor = egui::text::CCursor::new(char_idx);
                            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                            egui::TextEdit::store_state(ui.ctx(), edit_id, state);
                        }
                    }
                    
                    // Request focus back to editor
                    request_focus = true;
                    
                    // Clear request
                    ctx.set_shared_state("markdown_editor_goto_line", 0usize);
                }
            }
        }

        // Attempt to restore a previously saved TextEdit state (caret/selection) once on first render
        if self.text_edit_id.is_none() {
            if let Some(saved_state) = ctx.get_plugin_data::<TextEditState>("markdown-editor", "textedit_state") {
                egui::TextEdit::store_state(ui.ctx(), egui::Id::new("markdown_editor_textedit").with(tab_id), saved_state.clone());
                tracing::debug!("markdown-editor.render_editor: restored saved TextEditState (once)");
            }
        }

        // Check for focus request (external or internal from goto_line)
        if let Some(focus_requested) = ctx.get_shared_state::<bool>("markdown_editor_focus_requested") {
            if focus_requested {
                request_focus = true;
                // Clear the request
                ctx.set_shared_state("markdown_editor_focus_requested", false);
            }
        }

        let output = scroll_area.show(ui, |ui| {
                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut self.content)
                        .id(egui::Id::new("markdown_editor_textedit").with(tab_id))
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .lock_focus(request_focus)
                )
        });
        
        let response = output.inner;

        // Track last active tab for multi-tab coordination
        if response.has_focus() {
            ctx.set_shared_state("markdown_editor_last_active_tab", tab_id.to_string());
        }

        // If we requested focus, explicitly request it from memory as well to be sure
        if request_focus {
            ui.ctx().memory_mut(|m| m.request_focus(response.id));
        }

        // Clicking the widget should focus it via egui normally; log clicks for diagnostics
        if response.clicked() {
            tracing::debug!("markdown-editor.render_editor: editor clicked, should receive focus normally");
        }

        // Log input events when editor has focus, and detect text events without focus
        let editor_has_focus = response.has_focus();
        ui.ctx().input(|input| {
            if editor_has_focus {
                for ev in input.events.iter() {
                    match ev {
                        egui::Event::Key { key, pressed, .. } => {
                            tracing::debug!("markdown-editor.render_editor: key event: {:?}, pressed={}", key, pressed);
                        }
                        egui::Event::Text(text) => {
                            tracing::debug!("markdown-editor.render_editor: text event: {:?}", text);
                        }
                        egui::Event::Paste(text) => {
                            tracing::debug!("markdown-editor.render_editor: paste event: {:?}", text);
                        }
                        _ => {}
                    }
                }
            } else {
                let text_count = input.events.iter().filter(|ev| matches!(ev, egui::Event::Text(_))).count();
                if text_count > 0 {
                    tracing::debug!("markdown-editor.render_editor: {} text events received while editor not focused", text_count);
                }
            }
        });

        // Log when TextEdit id changes (useful to detect rebuilds)
        if self.text_edit_id.map(|id| id != response.id).unwrap_or(true) {
            tracing::debug!("markdown-editor.render_editor: TextEdit id changed: old={:?} new={:?}", self.text_edit_id, response.id);
        }

        // Store the ID for later state retrieval
        self.text_edit_id = Some(response.id);

        // Save TextEdit state (caret/selection) into plugin_data only when content or cursor changed
        if response.changed() {
            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                ctx.set_plugin_data("markdown-editor", "textedit_state", state.clone());
                tracing::debug!("markdown-editor.render_editor: saved TextEditState on change (len={})", self.content.len());
            }
        }

        // Also save when cursor moved (so caret position is preserved)
        if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
            if let Some(cursor) = state.cursor.char_range() {
                let cursor_idx = cursor.primary.index;
                if self.last_cursor_char_idx.map(|prev| prev != cursor_idx).unwrap_or(true) {
                    ctx.set_plugin_data("markdown-editor", "textedit_state", state.clone());
                    tracing::debug!("markdown-editor.render_editor: saved TextEditState on cursor move to {}", cursor_idx);
                }
            }
        }

        // Handle content changes
        if response.changed() {
            tracing::debug!("markdown-editor.render_editor: TextEdit changed (content_len={}), has_focus={}", self.content.len(), response.has_focus());
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

        // Publish cursor line and detect cursor movement
        if let Some(state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
            if let Some(cursor) = state.cursor.char_range() {
                 let cursor_idx = cursor.primary.index;
                 // Calculate line number (1-based)
                 let line = self.content.chars().take(cursor_idx).filter(|&c| c == '\n').count() + 1;
                 ctx.set_shared_state("markdown_editor_cursor_line", line);

                 // If the cursor moved, update last cursor index (title updates removed)
                 let cursor_changed = match self.last_cursor_char_idx {
                     Some(prev) => prev != cursor_idx,
                     None => true,
                 };
 
                 if cursor_changed {
                     tracing::debug!("markdown-editor.render_editor: cursor moved from {:?} to {}", self.last_cursor_char_idx, cursor_idx);
                     self.last_cursor_char_idx = Some(cursor_idx);
                 }

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
        
        // No heading found: return empty so UI can use default tab name
        String::new()
    }
}

/// Actions that can be performed on the dock state
enum DockAction {
    SplitHorizontal(SurfaceIndex, NodeIndex, String),
    SplitVertical(SurfaceIndex, NodeIndex, String),
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
            if self.core.current_title.is_empty() {
                tab.as_str().into()
            } else {
                self.core.current_title.clone().into()
            }
        } else {
            tab.as_str().into()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        self.core.render_editor(ui, self.ctx, tab);
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab, surface: SurfaceIndex, node: NodeIndex) {
        if ui.button("Split Horizontal").clicked() {
            *self.pending_action = Some(DockAction::SplitHorizontal(surface, node, tab.clone()));
            ui.close_menu();
        }
        if ui.button("Split Vertical").clicked() {
            *self.pending_action = Some(DockAction::SplitVertical(surface, node, tab.clone()));
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
            tracing::debug!("markdown-editor.initialize: received shared_state content (len={})", content.len());
            self.core.content = content;
            self.core.has_changes = false;
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
        
        // Sync inbound shared state content into editor if provided
        if let Some(in_content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            tracing::debug!("markdown-editor.update: found shared_state content (len={}), current_len={}", in_content.len(), self.core.content.len());
            if in_content != self.core.content {
                if self.core.has_changes {
                    tracing::debug!("markdown-editor.update: received external content but local changes exist; skipping apply");
                } else {
                    tracing::info!("markdown-editor.update: applying shared_state content to editor core");
                    self.core.content = in_content;
                    // Content loaded from disk should be treated as saved
                    self.core.has_changes = false;
                    self.core.update_stats();
                }
            }
        }

        // Also check plugin-specific data for loaded content (fallback channel)
        if let Some(loaded) = ctx.get_plugin_data::<String>("markdown-editor", "loaded_content") {
            tracing::debug!("markdown-editor.update: found plugin_data loaded_content (len={})", loaded.len());
            if loaded != self.core.content && !loaded.is_empty() {
                tracing::info!("markdown-editor.update: applying plugin_data loaded_content to editor core");
                self.core.content = loaded;
                self.core.has_changes = false;
                self.core.update_stats();
                ctx.set_plugin_data("markdown-editor", "loaded_content", String::new());
            }
        }

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
        
        // Sync inbound shared state content into editor if provided
        if let Some(in_content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            tracing::debug!("markdown-editor.panel_update: found shared_state content (len={}), current_len={}", in_content.len(), self.core.content.len());
            if in_content != self.core.content {
                if self.core.has_changes {
                    tracing::debug!("markdown-editor.panel_update: received external content but local changes exist; skipping apply");
                } else {
                    tracing::info!("markdown-editor.panel_update: applying shared_state content to editor core");
                    self.core.content = in_content;
                    // Content loaded from disk should be treated as saved
                    self.core.has_changes = false;
                    self.core.update_stats();
                }
            }
        }

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
        // Static title 'Editor' — dynamic title logic removed to avoid flicker/focus issues.
        // Update shared state for other plugins (like Outline)
        if self.core.has_changes {
            tracing::debug!("markdown-editor.render_panel: publishing shared_state content (len={})", self.core.content.len());
            ctx.set_shared_state("markdown_editor_content", self.core.content.clone());
        } else {
            // Also publish current content length for diagnostic purposes
            tracing::debug!("markdown-editor.render_panel: no changes (has_changes={}), content_len={}", self.core.has_changes, self.core.content.len());
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
            
            // Helper to copy state
            let copy_state = |ctx: &egui::Context, source_tab: &str, new_tab: &str| {
                let source_id = egui::Id::new("markdown_editor_textedit").with(source_tab);
                let new_id = egui::Id::new("markdown_editor_textedit").with(new_tab);
                
                if let Some(state) = egui::TextEdit::load_state(ctx, source_id) {
                    egui::TextEdit::store_state(ctx, new_id, state);
                }
            };

            match action {
                DockAction::SplitHorizontal(surface, node, source_tab) => {
                    copy_state(ui.ctx(), &source_tab, &new_tab);
                    self.tree.split((surface, node), Split::Right, 0.5, Node::leaf(new_tab.clone()));
                    // Request focus for the new tab
                    ctx.set_shared_state("markdown_editor_focus_requested", true);
                    // Also set it as active tab so it captures focus logic
                    ctx.set_shared_state("markdown_editor_last_active_tab", new_tab);
                }
                DockAction::SplitVertical(surface, node, source_tab) => {
                    copy_state(ui.ctx(), &source_tab, &new_tab);
                    self.tree.split((surface, node), Split::Below, 0.5, Node::leaf(new_tab.clone()));
                    // Request focus for the new tab
                    ctx.set_shared_state("markdown_editor_focus_requested", true);
                    // Also set it as active tab so it captures focus logic
                    ctx.set_shared_state("markdown_editor_last_active_tab", new_tab);
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

    #[test]
    fn test_update_syncs_shared_state() {
        let mut editor = MarkdownEditorPlugin::new();
        let mut ctx = PluginContext::new();

        // Shared state has content before update
        ctx.set_shared_state("markdown_editor_content", "Loaded content".to_string());

        // Call update which should sync shared state into the editor core
        assert!(cosmarium_plugin_api::Plugin::update(&mut editor, &mut ctx).is_ok());
        assert_eq!(editor.content(), "Loaded content");
        assert!(!editor.has_changes(), "Content loaded from shared state should be treated as saved");
    }
}
