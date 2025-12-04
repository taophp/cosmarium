//! Main application structure for Cosmarium.
//!
//! This module contains the main application state and UI logic, implementing
//! the eframe::App trait for the EGUI framework. It manages the plugin system,
//! layout management, and core application functionality.

use crate::AppArgs;
use cosmarium_core::{Application, PluginManager, Layout, LayoutManager, Config, Result};
use cosmarium_plugin_api::{Plugin, PluginContext, PanelPlugin, Event, EventType};
use cosmarium_markdown_editor::MarkdownEditorPlugin;
use eframe::egui;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use cosmarium_core::Session;

/// Main Cosmarium application state
pub struct Cosmarium {
    /// Core application instance
    core_app: Application,
    /// Plugin context for inter-plugin communication
    plugin_context: PluginContext,
    /// Currently loaded plugins
    plugins: HashMap<String, Box<dyn Plugin>>,
    /// Panel plugins for UI rendering
    panel_plugins: HashMap<String, Box<dyn PanelPlugin>>,
    /// Application configuration
    config: Config,
    /// Command line arguments
    args: AppArgs,
    /// User session data
    session: Session,
    /// Application startup time
    startup_time: Instant,
    /// Whether to show the about dialog
    show_about: bool,
    /// Whether to show plugin manager
    show_plugin_manager: bool,
    /// Whether to show settings dialog
    show_settings: bool,
    /// Current project path
    current_project: Option<std::path::PathBuf>,
    /// Active document being edited
    active_document_id: Option<uuid::Uuid>,
    /// Recent projects cache
    recent_projects: Vec<std::path::PathBuf>,
    /// Current Git branch
    current_branch: Option<String>,
    /// UI state
    ui_state: UiState,
    /// Whether to show the new project dialog
    show_new_project_dialog: bool,
    /// New project dialog state
    new_project_name: String,
    new_project_path: String,
    new_project_template: String,
}

/// Identifiers for the top-level menus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuId {
    Cosmarium,
    File,
    Edit,
    View,
    Tools,
    Help,
}

/// UI state for the application
#[derive(Debug, Clone)]
struct UiState {
    /// Which panels are currently open
    open_panels: HashMap<String, bool>,
    /// Left panel width
    left_panel_width: f32,
    /// Right panel width
    right_panel_width: f32,
    /// Bottom panel height
    bottom_panel_height: f32,
    /// Whether menu bar is visible
    show_menu_bar: bool,
    /// Whether status bar is visible
    show_status_bar: bool,
    /// Whether any menu is currently expanded
    menu_expanded: bool,
    /// The name of the menu currently hovered over
    hovered_menu: Option<String>,
    /// The currently active (open) menu
    active_menu: Option<MenuId>,
    /// Current theme name
    current_theme: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            open_panels: HashMap::new(),
            left_panel_width: 250.0,
            right_panel_width: 300.0,
            bottom_panel_height: 200.0,
            show_menu_bar: true,
            show_status_bar: true,
            menu_expanded: false,
            hovered_menu: None,
            active_menu: None,
            current_theme: "Dark".to_string(),
        }
    }
}

impl Cosmarium {
    /// Create a new Cosmarium application instance.
    ///
    /// # Arguments
    ///
    /// * `cc` - eframe creation context
    /// * `args` - Command line arguments
    pub fn new(cc: &eframe::CreationContext<'_>, args: AppArgs) -> Self {
        let mut app = Self {
            core_app: Application::new(),
            plugin_context: PluginContext::new(),
            plugins: HashMap::new(),
            panel_plugins: HashMap::new(),
            config: Config::default(),
            args,
            session: Session::load(),
            startup_time: Instant::now(),
            show_about: false,
            show_plugin_manager: false,
            show_settings: false,
            current_project: None,
            active_document_id: None,
            recent_projects: Vec::new(), // Will be populated from session
            current_branch: None,
            ui_state: UiState::default(),
            show_new_project_dialog: false,
            new_project_name: String::new(),
            new_project_path: dirs::document_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("Cosmarium Projects")
                .to_string_lossy()
                .to_string(),
            new_project_template: "novel".to_string(),
        };

        // Initialize the application
        if let Err(e) = app.initialize() {
            tracing::error!("Failed to initialize application: {}", e);
        }

        // Load project if specified in args (clone the Option to avoid borrowing `app` across a mutable borrow)
        if let Some(project_path) = app.args.project_path.clone() {
            if let Err(e) = app.open_project_async(project_path) {
                tracing::error!("Failed to open project from args: {}", e);
            }
        } else if app.config.app.restore_session {
            // Auto-open last project if enabled and no project specified in args
            if let Some(last_project) = app.session.last_opened_project.clone() {
                if last_project.exists() {
                    if let Err(e) = app.open_project_async(last_project) {
                        tracing::error!("Failed to restore last session: {}", e);
                    }
                }
            }
        }
        
        // Sync recent projects from session to app state (for UI access)
        app.recent_projects = app.session.recent_projects.clone();

        app
    }

    /// Initialize the application and load core plugins.
    fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing Cosmarium application");

        // Initialize core application (this is async, so we need a runtime)
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        rt.block_on(async {
            self.core_app.initialize().await
        })?;

        // Load recent projects
        let project_manager = Arc::clone(&self.core_app.project_manager());
        let recent = rt.block_on(async {
            let pm = project_manager.read().await;
            pm.recent_projects().to_vec()
        });
        self.recent_projects = recent;

        // Load configuration
        self.config = Config::load_or_default()?;

        // Initialize core plugins
        self.load_core_plugins()?;

        // Emit application startup event
        let event = Event::new(
            EventType::ApplicationStartup,
            format!("Cosmarium v{} started", env!("CARGO_PKG_VERSION"))
        );
        self.plugin_context.emit_event(event);

        tracing::info!("Application initialized in {:?}", self.startup_time.elapsed());
        Ok(())
    }

    /// Load core plugins that are essential for basic functionality.
    fn load_core_plugins(&mut self) -> Result<()> {
        // Load markdown editor plugin
        let mut markdown_editor = MarkdownEditorPlugin::new();
        markdown_editor.initialize(&mut self.plugin_context)?;
        
        let plugin_name = markdown_editor.info().name.clone();
        self.panel_plugins.insert(plugin_name.clone(), Box::new(markdown_editor));
        
        // Mark the editor panel as open by default
        self.ui_state.open_panels.insert(plugin_name, true);

        tracing::info!("Core plugins loaded");
        Ok(())
    }

    /// Get the current Git branch name if a project is open.
    fn get_current_branch(&self) -> Option<String> {
        let project_manager = Arc::clone(&self.core_app.project_manager());
        
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(async {
            let pm = project_manager.read().await;
            if let Some(project) = pm.active_project() {
                if let Some(git) = project.git() {
                    return git.current_branch().ok();
                }
            }
            None
        })
    }

    /// Load a project from the specified path.
    fn load_project(&mut self, path: &std::path::Path) -> Result<()> {
        tracing::info!("Loading project from {:?}", path);
        
        // TODO: Implement actual project loading
        self.current_project = Some(path.to_path_buf());
        
        let event = Event::new(
            EventType::ProjectOpened,
            format!("Opened project: {}", path.display())
        );
        self.plugin_context.emit_event(event);
        
        Ok(())
    }

    /// Open a project asynchronously (called from file dialog).
    fn open_project_async(&mut self, path: std::path::PathBuf) -> Result<()> {
        tracing::info!("Opening project from {:?}", path);
        
        // Clone the Arc to avoid lifetime issues
        let project_manager = Arc::clone(&self.core_app.project_manager());
        let path_clone = path.clone();
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        rt.block_on(async move {
            let mut pm = project_manager.write().await;
            pm.open_project(&path_clone).await
        })?;
        
        self.current_project = Some(path.clone());
        
        // Load first document into editor (if any)
        let project_manager = Arc::clone(&self.core_app.project_manager());
        let document_manager = Arc::clone(&self.core_app.document_manager());
        
        let rt2 = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        let pm_clone = Arc::clone(&project_manager);
        let (doc_id_opt, doc_content) = rt2.block_on(async move {
            let pm = pm_clone.read().await;
            let dm = document_manager.read().await;
            
            if let Some(project) = pm.active_project() {
                if let Some(&doc_id) = project.documents().first() {
                    if let Some(doc) = dm.get_document(doc_id) {
                        return (Some(doc_id), Some(doc.content().to_string()));
                    }
                }
            }
            (None, None)
        });
        
        // Set active document and content in editor if we got any
        self.active_document_id = doc_id_opt;
        if let Some(content) = doc_content {
            self.plugin_context.set_shared_state("markdown_editor_content", content);
        }
        
        // Update recent projects list
        let rt3 = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        let recent = rt3.block_on(async {
            let pm = project_manager.read().await;
            pm.recent_projects().to_vec()
        });
        self.recent_projects = recent;
        
        // Get current Git branch
        self.current_branch = self.get_current_branch();

        // Update session
        self.session.add_recent_project(path.clone(), self.config.app.max_recent_projects);
        self.session.last_opened_project = Some(path);
        if let Err(e) = self.session.save() {
            tracing::warn!("Failed to save session: {}", e);
        }
        
        // Update UI list
        self.recent_projects = self.session.recent_projects.clone();
        
        Ok(())
    }

    /// Save the current project.
    fn save_current_project(&mut self) -> Result<()> {
        if self.current_project.is_none() {
            tracing::warn!("No active project to save");
            return Ok(());
        }
        
        tracing::info!("Saving current project");
        
        // Get editor content from MarkdownEditorPlugin
        let editor_content = if let Some(editor) = self.panel_plugins.get("Markdown Editor") {
            // Try to downcast to MarkdownEditorPlugin
            // Since we can't easily downcast trait objects, use shared state
            self.plugin_context.get_shared_state::<String>("markdown_editor_content")
        } else {
            None
        };
        
        let project_manager = Arc::clone(&self.core_app.project_manager());
        let document_manager = Arc::clone(&self.core_app.document_manager());
        let document_id = self.active_document_id;
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        
        // Save editor content to document if we have content
        if let Some(content) = editor_content {
            let saved_doc_id = rt.block_on(async move {
                let mut dm = document_manager.write().await;
                let mut pm = project_manager.write().await;
                
                // Create or update document
                let doc_id = if let Some(id) = document_id {
                    // Update existing document
                    if let Some(doc) = dm.get_document_mut(id) {
                        doc.set_content(&content);
                        dm.save_document(id).await?;
                    }
                    id
                } else {
                    // Create new document
                    use cosmarium_core::document::DocumentFormat;
                    let doc_id = dm.create_document("Untitled", &content, DocumentFormat::Markdown).await?;
                    
                    // Add document to project
                    if let Some(project) = pm.active_project_mut() {
                        project.add_document(doc_id);
                    }
                    
                    doc_id
                };
                
                // Save project
                pm.save_project().await?;
                
                Ok::<Uuid, anyhow::Error>(doc_id)
            })?;
            
            // Update active document ID
            self.active_document_id = Some(saved_doc_id);
        } else {
            // No editor content, just save project
            rt.block_on(async move {
                let mut pm = project_manager.write().await;
                pm.save_project().await
            })?;
        }
        
        Ok(())
    }

    /// Create a new project with the given parameters.
    fn create_new_project(&mut self, name: String, path: String, template: String) -> Result<()> {
        let project_path = std::path::PathBuf::from(&path).join(&name);
        
        tracing::info!("Creating new project '{}' at {:?}", name, project_path);
        
        let project_manager = Arc::clone(&self.core_app.project_manager());
        let project_path_clone = project_path.clone();
        let path_buf = project_path.clone();
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        rt.block_on(async move {
            let mut pm = project_manager.write().await;
            pm.create_project(&name, &path_buf, &template).await
        })?;
        
        self.current_project = Some(project_path.clone());
        
        // Update recent projects list
        let rt2 = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;
        let project_manager = Arc::clone(&self.core_app.project_manager());
        let recent = rt2.block_on(async {
            let pm = project_manager.read().await;
            pm.recent_projects().to_vec()
        });
        self.recent_projects = recent;
        
        // Get current Git branch
        self.current_branch = self.get_current_branch();

        // Update session
        self.session.add_recent_project(project_path.clone(), self.config.app.max_recent_projects);
        self.session.last_opened_project = Some(project_path);
        if let Err(e) = self.session.save() {
            tracing::warn!("Failed to save session: {}", e);
        }
        self.recent_projects = self.session.recent_projects.clone();
        
        Ok(())
    }

    /// Render the main menu bar.
    /// Render the menu bar (Zed-style: compact or expanded).
    fn render_menu_bar(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.ui_state.menu_expanded {
            self.render_expanded_menu_bar(ctx);
        } else {
            self.render_compact_menu_bar(ctx);
        }
    }

    /// Render the compact menu bar (burger + project + branch).
    fn render_compact_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("compact_menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Configure visuals for "frameless feel but visible hover"
                let mut visuals = ui.visuals().clone();
                visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                
                visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
                
                ui.ctx().set_visuals(visuals);

                // Burger menu button
                if ui.button("☰").clicked() {
                    self.ui_state.menu_expanded = true;
                    self.ui_state.active_menu = Some(MenuId::Cosmarium);
                }
                
                // Project name button (now a submenu)
                let project_name = self.current_project
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("No Project")
                    .to_string();
                    
                ui.menu_button(project_name, |ui| {
                    ui.set_min_width(200.0);
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                        if self.recent_projects.is_empty() {
                            ui.label("No recent projects");
                        } else {
                            let mut path_to_open = None;
                            for path in &self.recent_projects {
                                let label = path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("Unknown Project");
                                    
                                if ui.button(label).clicked() {
                                    path_to_open = Some(path.clone());
                                    ui.close_menu();
                                }
                            }
                            
                            if let Some(path) = path_to_open {
                                if let Err(e) = self.open_project_async(path) {
                                    tracing::error!("Failed to open recent project: {}", e);
                                }
                            }
                        }
                        
                        ui.separator();
                        
                        if ui.button("New").clicked() {
                            ui.close_menu();
                            self.show_new_project_dialog = true;
                        }
                        
                        if ui.button("Open").clicked() {
                            ui.close_menu();
                            // Open project selector
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Open Project")
                                .pick_folder()
                            {
                                if let Err(e) = self.open_project_async(path) {
                                    tracing::error!("Failed to open project: {}", e);
                                }
                            }
                        }
                    });
                });
            });
        });
    }

    /// Render the expanded menu bar (custom implementation).
    fn render_expanded_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("expanded_menu_bar").show(ctx, |ui| {
            // Horizontal menu bar
            ui.horizontal(|ui| {
                // Configure visuals for "frameless feel but visible hover"
                let mut visuals = ui.visuals().clone();
                visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                
                visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
                
                visuals.widgets.noninteractive.bg_stroke.width = 0.5;
                visuals.window_stroke.width = 0.5;
                
                ui.ctx().set_visuals(visuals);

                // Helper closure to render a menu item
                let mut render_menu_item = |ui: &mut egui::Ui, id: MenuId, label: &str, content: Box<dyn FnOnce(&mut Self, &mut egui::Ui)>| {
                    let button_response = ui.button(label);
                    
                    // Click logic: toggle menu
                    if button_response.clicked() {
                        if self.ui_state.active_menu == Some(id) {
                            self.ui_state.active_menu = None;
                            self.ui_state.menu_expanded = false;
                        } else {
                            self.ui_state.active_menu = Some(id);
                        }
                    }
                    
                    // Hover logic: switch if another menu is open
                    if self.ui_state.active_menu.is_some() && button_response.hovered() {
                        self.ui_state.active_menu = Some(id);
                    }
                    
                    // Popup rendering
                    if self.ui_state.active_menu == Some(id) {
                        let popup_id = ui.make_persistent_id(format!("{}_popup", label));
                        
                        // Sync with egui's internal state to ensure popup_below_widget renders
                        if !ui.memory(|m| m.is_popup_open(popup_id)) {
                            ui.memory_mut(|m| m.open_popup(popup_id));
                        }
                        
                        egui::popup::popup_below_widget(ui, popup_id, &button_response, |ui| {
                            ui.set_min_width(150.0);
                            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                                content(self, ui);
                            });
                        });
                    }
                };

                // Cosmarium Menu
                render_menu_item(ui, MenuId::Cosmarium, "Cosmarium", Box::new(|app, ui| {
                    if ui.button("About").clicked() {
                        app.show_about = true;
                        app.ui_state.menu_expanded = false;
                        app.ui_state.active_menu = None;
                    }
                    if ui.button("Settings").clicked() {
                        app.show_settings = true;
                        app.ui_state.menu_expanded = false;
                        app.ui_state.active_menu = None;
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }));

                // File Menu
                render_menu_item(ui, MenuId::File, "File", Box::new(|app, ui| {
                    if ui.button("New Project").clicked() {
                        app.show_new_project_dialog = true;
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                    if ui.button("Open Project").clicked() {
                        // We need to close the menu before opening the dialog to avoid UI glitches
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                        
                        // Use a separate thread or deferred action for file dialog if possible, 
                        // but here we just call it. Note: rfd might block.
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Open Project")
                            .pick_folder()
                        {
                            if let Err(e) = app.open_project_async(path) {
                                tracing::error!("Failed to open project: {}", e);
                            }
                        }
                    }
                    


                    if ui.button("Save Project").clicked() {
                        if let Err(e) = app.save_current_project() {
                            tracing::error!("Failed to save project: {}", e);
                        }
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                    ui.separator();
                    if ui.button("Export...").clicked() {
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                }));

                // Edit Menu
                render_menu_item(ui, MenuId::Edit, "Edit", Box::new(|app, ui| {
                    if ui.button("Undo").clicked() { app.ui_state.active_menu = None; app.ui_state.menu_expanded = false; }
                    if ui.button("Redo").clicked() { app.ui_state.active_menu = None; app.ui_state.menu_expanded = false; }
                    ui.separator();
                    if ui.button("Cut").clicked() { app.ui_state.active_menu = None; app.ui_state.menu_expanded = false; }
                    if ui.button("Copy").clicked() { app.ui_state.active_menu = None; app.ui_state.menu_expanded = false; }
                    if ui.button("Paste").clicked() { app.ui_state.active_menu = None; app.ui_state.menu_expanded = false; }
                }));

                // View Menu
                render_menu_item(ui, MenuId::View, "View", Box::new(|app, ui| {
                    // We need to collect changes to avoid borrowing issues
                    let mut panels_to_toggle = Vec::new();
                    
                    for (panel_name, panel_plugin) in &app.panel_plugins {
                        let is_open = app.ui_state.open_panels.get(panel_name).unwrap_or(&false);
                        let mut open = *is_open;
                        
                        if ui.checkbox(&mut open, panel_plugin.panel_title()).clicked() {
                            panels_to_toggle.push((panel_name.clone(), open));
                            // Don't close menu for checkboxes usually
                        }
                    }
                    
                    for (name, open) in panels_to_toggle {
                        app.ui_state.open_panels.insert(name, open);
                    }
                    
                    ui.separator();
                    
                    if ui.checkbox(&mut app.ui_state.show_menu_bar, "Menu Bar").clicked() {
                        // app.ui_state.active_menu = None; // Optional
                    }
                    if ui.checkbox(&mut app.ui_state.show_status_bar, "Status Bar").clicked() {
                        // app.ui_state.active_menu = None; // Optional
                    }
                }));

                // Tools Menu
                render_menu_item(ui, MenuId::Tools, "Tools", Box::new(|app, ui| {
                    if ui.button("Plugin Manager").clicked() {
                        app.show_plugin_manager = true;
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                    if ui.button("Settings").clicked() {
                        app.show_settings = true;
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                }));

                // Help Menu
                render_menu_item(ui, MenuId::Help, "Help", Box::new(|app, ui| {
                    if ui.button("About Cosmarium").clicked() {
                        app.show_about = true;
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                    if ui.button("Documentation").clicked() {
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                    if ui.button("Report Issue").clicked() {
                        app.ui_state.active_menu = None;
                        app.ui_state.menu_expanded = false;
                    }
                }));

            });
        });
        
        // Detect click outside menu to close
        if self.ui_state.active_menu.is_some() {
            ctx.input(|i| {
                if i.pointer.any_click() {
                    // Check if click is outside the top panel
                    if let Some(pos) = i.pointer.interact_pos() {
                        // Simple heuristic: if click is below y=100, close menu
                        // (menu bar is typically at the top)
                        if pos.y > 100.0 {
                            self.ui_state.active_menu = None;
                            self.ui_state.menu_expanded = false;
                        }
                    }
                }
            });
        }
    }


    /// Render the status bar.
    fn render_status_bar(&mut self, ctx: &egui::Context) {
        if !self.ui_state.show_status_bar {
            return;
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Project info
                if let Some(ref project) = self.current_project {
                    ui.label(format!("📁 {}", project.file_stem().unwrap_or_default().to_string_lossy()));
                    ui.separator();
                } else {
                    ui.label("📝 No project");
                    ui.separator();
                }

                // Plugin status
                ui.label(format!("🔌 {} plugins", self.panel_plugins.len()));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Application info
                    ui.label(format!("Cosmarium v{}", env!("CARGO_PKG_VERSION")));
                });
            });
        });
    }

    /// Render panels based on their position.
    fn render_panels(&mut self, ctx: &egui::Context) {
        // Left panel
        if self.has_panels_in_position(cosmarium_plugin_api::PanelPosition::Left) {
            egui::SidePanel::left("left_panel")
                .width_range(200.0..=400.0)
                .default_width(self.ui_state.left_panel_width)
                .show(ctx, |ui| {
                    self.render_panels_in_position(ui, cosmarium_plugin_api::PanelPosition::Left);
                });
        }

        // Right panel
        if self.has_panels_in_position(cosmarium_plugin_api::PanelPosition::Right) {
            egui::SidePanel::right("right_panel")
                .width_range(200.0..=400.0)
                .default_width(self.ui_state.right_panel_width)
                .show(ctx, |ui| {
                    self.render_panels_in_position(ui, cosmarium_plugin_api::PanelPosition::Right);
                });
        }

        // Bottom panel
        if self.has_panels_in_position(cosmarium_plugin_api::PanelPosition::Bottom) {
            egui::TopBottomPanel::bottom("bottom_panel")
                .height_range(100.0..=300.0)
                .default_height(self.ui_state.bottom_panel_height)
                .show(ctx, |ui| {
                    self.render_panels_in_position(ui, cosmarium_plugin_api::PanelPosition::Bottom);
                });
        }

        // Central panel (main content area)
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_panels_in_position(ui, cosmarium_plugin_api::PanelPosition::Center);
        });
    }

    /// Check if there are any panels in the specified position that should be shown.
    fn has_panels_in_position(&self, position: cosmarium_plugin_api::PanelPosition) -> bool {
        self.panel_plugins.iter().any(|(name, plugin)| {
            plugin.default_position() == position &&
            *self.ui_state.open_panels.get(name).unwrap_or(&false)
        })
    }

    /// Render all panels in the specified position.
    fn render_panels_in_position(&mut self, ui: &mut egui::Ui, position: cosmarium_plugin_api::PanelPosition) {
        let panels_to_render: Vec<String> = self.panel_plugins.iter()
            .filter_map(|(name, plugin)| {
                if plugin.default_position() == position &&
                   *self.ui_state.open_panels.get(name).unwrap_or(&false) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        // If no panels are open in center position, show a friendly message and return early.
        if panels_to_render.is_empty() && position == cosmarium_plugin_api::PanelPosition::Center {
            ui.centered_and_justified(|ui| {
                ui.label("No panels open. Use the plugin manager to enable panels.");
            });
            return;
        }

        for panel_name in &panels_to_render {
            if let Some(panel) = self.panel_plugins.get_mut(panel_name) {
                // Create a collapsing header for each panel
                let title = panel.panel_title().to_string();
                let header_response = ui.collapsing(title, |ui| {
                    panel.render_panel(ui, &mut self.plugin_context);
                });

                // Handle panel closing if closable
                if panel.is_closable() {
                    header_response.header_response.context_menu(|ui| {
                        if ui.button("Close Panel").clicked() {
                            self.ui_state.open_panels.insert(panel_name.clone(), false);
                            ui.close_menu();
                        }
                    });
                }
            }
        }



    }

    /// Render modal dialogs.
    fn render_dialogs(&mut self, ctx: &egui::Context) {
        // About dialog
        if self.show_about {
            egui::Window::new("About Cosmarium")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Cosmarium");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.separator();
                        ui.label("Next-generation creative writing software");
                        ui.label("for fiction authors");
                        ui.separator();
                        ui.label("Built with Rust and EGUI");
                        ui.separator();
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }

        // Plugin manager dialog
        if self.show_plugin_manager {
            egui::Window::new("Plugin Manager")
                .collapsible(false)
                .default_width(600.0)
                .show(ctx, |ui| {
                    ui.label("Installed Plugins:");
                    ui.separator();
                    
                    for (name, plugin) in &self.panel_plugins {
                        ui.horizontal(|ui| {
                            ui.label("🔌");
                            ui.label(name);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let is_open = self.ui_state.open_panels.get(name).unwrap_or(&false);
                                if *is_open {
                                    ui.colored_label(egui::Color32::GREEN, "Active");
                                } else {
                                    ui.colored_label(egui::Color32::GRAY, "Inactive");
                                }
                            });
                        });
                        ui.separator();
                    }
                    
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_plugin_manager = false;
                        }
                    });
                });
        }

        // Settings dialog
        if self.show_settings {
            egui::Window::new("Settings")
                .collapsible(false)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.label("Application Settings");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        egui::ComboBox::from_label("")
                            .selected_text(&self.ui_state.current_theme)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.ui_state.current_theme, "Dark".to_string(), "Dark");
                                ui.selectable_value(&mut self.ui_state.current_theme, "Light".to_string(), "Light");
                            });
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            // TODO: Save settings
                            self.show_settings = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_settings = false;
                        }
                    });
                });
        }

        // New Project dialog
        if self.show_new_project_dialog {
            egui::Window::new("New Project")
                .collapsible(false)
                .default_width(450.0)
                .show(ctx, |ui| {
                    ui.label("Create a new Cosmarium project");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Project Name:");
                        ui.text_edit_singleline(&mut self.new_project_name);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Location:");
                        ui.text_edit_singleline(&mut self.new_project_path);
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Select Project Location")
                                .pick_folder()
                            {
                                self.new_project_path = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Template:");
                        egui::ComboBox::from_label("")
                            .selected_text(&self.new_project_template)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.new_project_template, "novel".to_string(), "Novel");
                                ui.selectable_value(&mut self.new_project_template, "short-story".to_string(), "Short Story");
                                ui.selectable_value(&mut self.new_project_template, "screenplay".to_string(), "Screenplay");
                                ui.selectable_value(&mut self.new_project_template, "blog".to_string(), "Blog");
                            });
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            if !self.new_project_name.is_empty() {
                                if let Err(e) = self.create_new_project(
                                    self.new_project_name.clone(),
                                    self.new_project_path.clone(),
                                    self.new_project_template.clone(),
                                ) {
                                    tracing::error!("Failed to create project: {}", e);
                                } else {
                                    self.show_new_project_dialog = false;
                                    self.new_project_name.clear();
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_new_project_dialog = false;
                            self.new_project_name.clear();
                        }
                    });
                });
        }
    }
}

impl eframe::App for Cosmarium {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Update plugins
        for plugin in self.plugins.values_mut() {
            if let Err(e) = plugin.update(&mut self.plugin_context) {
                tracing::error!("Plugin update error: {}", e);
            }
        }

        // Keyboard shortcuts (Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Q)
        // Keyboard shortcuts (Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Q)
        let mut should_quit = false;
        ctx.input(|input| {
            if input.modifiers.ctrl {
                if input.key_pressed(egui::Key::N) {
                    // New project
                    self.show_new_project_dialog = true;
                } else if input.key_pressed(egui::Key::O) {
                    // Open project (reuse file dialog logic)
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Project")
                        .pick_folder()
                    {
                        if let Err(e) = self.open_project_async(path) {
                            tracing::error!("Failed to open project via shortcut: {}", e);
                        }
                    }
                } else if input.key_pressed(egui::Key::S) {
                    // Save current project
                    if let Err(e) = self.save_current_project() {
                        tracing::error!("Failed to save project via shortcut: {}", e);
                    }
                } else if input.key_pressed(egui::Key::Q) {
                    // Quit application
                    should_quit = true;
                } else if input.key_pressed(egui::Key::Z) {
                    // Undo
                    if input.modifiers.shift {
                        // Redo (Ctrl+Shift+Z)
                        self.plugin_context.set_shared_state("markdown_editor_action", "redo".to_string());
                    } else {
                        // Undo (Ctrl+Z)
                        self.plugin_context.set_shared_state("markdown_editor_action", "undo".to_string());
                    }
                } else if input.key_pressed(egui::Key::Y) {
                    // Redo (Ctrl+Y)
                    self.plugin_context.set_shared_state("markdown_editor_action", "redo".to_string());
                }
            }
        });

        if should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }


        // Render UI
        if self.ui_state.show_menu_bar {
            self.render_menu_bar(ctx, frame);
        }
        
        self.render_status_bar(ctx);
        self.render_panels(ctx);
        self.render_dialogs(ctx);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        tracing::info!("Saving application state");
        
        // TODO: Save application state and configuration
        if let Err(e) = self.config.save() {
            tracing::error!("Failed to save configuration: {}", e);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        tracing::info!("Application shutting down");
        
        // Emit shutdown event
        let event = Event::new(EventType::ApplicationShutdown, "Application shutting down");
        self.plugin_context.emit_event(event);
        
        // Shutdown plugins
        for plugin in self.plugins.values_mut() {
            if let Err(e) = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(plugin.shutdown(&mut self.plugin_context))
            {
                tracing::error!("Plugin shutdown error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_state_default() {
        let ui_state = UiState::default();
        assert!(ui_state.show_menu_bar);
        assert!(ui_state.show_status_bar);
        assert_eq!(ui_state.current_theme, "Dark");
        assert_eq!(ui_state.left_panel_width, 250.0);
    }

    #[test]
    fn test_cosmarium_creation() {
        // This test would require mocking eframe::CreationContext
        // For now, we just verify the struct can be constructed
        let args = AppArgs::default();
        assert!(args.project_path.is_none());
    }
}