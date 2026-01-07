//! Main application structure for Cosmarium.
//!
//! This module contains the main application state and UI logic, implementing
//! the eframe::App trait for the EGUI framework. It manages the plugin system,
//! layout management, and core application functionality.

use crate::AppArgs;
use cosmarium_atmosphere::AtmospherePlugin;
use cosmarium_core::Session;
use cosmarium_core::{Application, Config, Layout, LayoutManager, PluginManager, Result};
use cosmarium_markdown_editor::MarkdownEditorPlugin;
use cosmarium_outline::OutlinePlugin;
use cosmarium_plugin_api::{Event, EventType, PanelPlugin, Plugin, PluginContext};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Main Cosmarium application state
pub struct Cosmarium {
    /// Core application instance
    core_app: Application,
    /// Plugin context for inter-plugin communication
    plugin_context: PluginContext,
    /// Current smoothed sentiment value (for direction/fallback)
    current_sentiment: f32,
    /// Current smoothed intensity value (for transition strength)
    current_intensity: f32,
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
    /// Whether to show the close confirmation dialog
    show_close_confirmation: bool,
    /// Whether to force close the application (ignoring unsaved changes)
    force_close: bool,
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
    /// Currently active panel in the left sidebar
    active_left_panel: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            open_panels: HashMap::new(),
            left_panel_width: 250.0,
            active_left_panel: None,
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
        // Enable IME for dead keys support
        cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::IMEAllowed(true));

        let mut app = Self {
            core_app: Application::new(),
            plugin_context: PluginContext::new(),
            current_sentiment: 0.0,
            current_intensity: 0.0,
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
            show_close_confirmation: false,
            force_close: false,
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
        rt.block_on(async { self.core_app.initialize().await })?;

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
            format!("Cosmarium v{} started", env!("CARGO_PKG_VERSION")),
        );
        self.plugin_context.emit_event(event);

        tracing::info!(
            "Application initialized in {:?}",
            self.startup_time.elapsed()
        );
        Ok(())
    }

    /// Load core plugins that are essential for basic functionality.
    fn load_core_plugins(&mut self) -> Result<()> {
        // Load markdown editor plugin
        let mut markdown_editor = MarkdownEditorPlugin::new();
        markdown_editor.initialize(&mut self.plugin_context)?;

        let plugin_name = markdown_editor.info().name.clone();
        self.panel_plugins
            .insert(plugin_name.clone(), Box::new(markdown_editor));

        // Mark the editor panel as open by default
        self.ui_state.open_panels.insert(plugin_name, true);

        // Load outline plugin
        let mut outline_plugin = OutlinePlugin::new();
        outline_plugin.initialize(&mut self.plugin_context)?;

        let outline_plugin_name = outline_plugin.info().name.clone();
        self.panel_plugins
            .insert(outline_plugin_name.clone(), Box::new(outline_plugin));
        // Outline panel is not open by default, so no need to add to open_panels

        // Load atmosphere plugin
        let mut atmosphere_plugin = AtmospherePlugin::new();
        atmosphere_plugin.initialize(&mut self.plugin_context)?;

        let atmosphere_name = atmosphere_plugin.info().name.clone();
        self.plugins
            .insert(atmosphere_name, Box::new(atmosphere_plugin));

        tracing::info!("Core plugins loaded. Total plugins: {}", self.plugins.len());
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
            format!("Opened project: {}", path.display()),
        );
        self.plugin_context.emit_event(event);

        Ok(())
    }

    /// Synchronize content from the editor plugin to the document manager
    fn sync_editor_content(&mut self) {
        let document_manager = self.core_app.document_manager();

        // Check if there's new content from the editor
        if let Some(content) = self
            .plugin_context
            .get_shared_state::<String>("markdown_editor_content")
        {
            if let Some(doc_id) = self.active_document_id {
                // Use a blocking runtime to acquire the write lock deterministically
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("Failed to create Tokio runtime for sync: {}", e);
                        return;
                    }
                };
                rt.block_on(async {
                    let mut manager = document_manager.write().await;
                    if let Some(doc) = manager.get_document_mut(doc_id) {
                        if doc.content() != content {
                            doc.set_content(&content);
                        }
                    }
                });
            }
        }
    }

    /// Check if there are any unsaved changes in the project
    fn check_unsaved_changes(&self) -> bool {
        // Use a blocking runtime to acquire read locks deterministically. If runtime
        // creation fails, be conservative and report unsaved changes.
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(
                    "Failed to create Tokio runtime for check_unsaved_changes: {}",
                    e
                );
                return true; // Conservative: assume there are unsaved changes
            }
        };

        let project_manager = self.core_app.project_manager();
        if rt.block_on(async {
            let manager = project_manager.read().await;
            if let Some(project) = manager.active_project() {
                return project.has_unsaved_changes();
            }
            false
        }) {
            return true;
        }

        let document_manager = self.core_app.document_manager();
        let any_unsaved = rt.block_on(async {
            let manager = document_manager.read().await;
            for doc_id in manager.list_documents() {
                if let Some(doc) = manager.get_document(doc_id) {
                    if doc.has_unsaved_changes() {
                        return true;
                    }
                }
            }
            false
        });

        any_unsaved
    }

    /// Save the current project.
    fn save_current_project(&mut self) -> Result<()> {
        // Sync editor content first
        self.sync_editor_content();

        // Capture editor content (if any) before entering async block
        let editor_content = self
            .plugin_context
            .get_shared_state::<String>("markdown_editor_content");

        let project_manager = self.core_app.project_manager();
        let document_manager = self.core_app.document_manager();

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?;

        // Save active document (if any) first, then save project metadata
        rt.block_on(async {
            // Determine project path if available
            let project_path_opt = {
                let pm_read = project_manager.read().await;
                pm_read.active_project().map(|p| p.path().to_path_buf()).or_else(|| self.current_project.clone())
            };

            tracing::debug!("save_current_project: project_path={:?}, active_document_id={:?}, editor_content_present={}",
                project_path_opt,
                self.active_document_id,
                editor_content.is_some());

            if let Some(doc_id) = self.active_document_id {
                // Acquire document manager write lock to modify document and set file path if missing
                let mut dm = document_manager.write().await;
                if let Some(doc) = dm.get_document_mut(doc_id) {
                    tracing::debug!("Found active document {} title='{}' file_path={:?}", doc_id, doc.title(), doc.file_path());

                    if doc.file_path().is_none() {
                        if let Some(proj_path) = &project_path_opt {
                            // Ensure content dir exists
                            let content_dir = proj_path.join("content");
                            if let Err(e) = tokio::fs::create_dir_all(&content_dir).await {
                                tracing::error!("Failed to create content dir: {}", e);
                            } else {
                                // Create filename from title or uuid fallback
                                let safe_name = if !doc.title().is_empty() { doc.title().to_lowercase().replace(" ", "_") } else { format!("doc_{}", uuid::Uuid::new_v4()) };
                                let filename = format!("{}.{}", safe_name, doc.format().extension());
                                let file_path = content_dir.join(filename);
                                tracing::debug!("Setting file path for doc {} -> {:?}", doc_id, file_path);
                                doc.set_file_path(&file_path);
                            }
                        } else {
                            tracing::warn!("No active project path; cannot set file path for document {}", doc_id);
                        }
                    }

                    let file_path_opt = doc.file_path().map(|p| p.to_path_buf());

                    if let Err(e) = dm.save_document(doc_id).await {
                        tracing::error!("Failed to save active document {}: {}", doc_id, e);
                    } else {
                        tracing::info!("Saved active document {} to {:?}", doc_id, file_path_opt);
                        // Check file metadata
                        if let Some(ref path) = file_path_opt {
                            match tokio::fs::metadata(path).await {
                                Ok(meta) => tracing::debug!("Saved file metadata: exists={}, size={}", true, meta.len()),
                                Err(e) => tracing::warn!("Saved file metadata not found for {:?}: {}", path, e),
                            }
                        }

                        // Ensure project references this document id before saving project
                        let mut pm = project_manager.write().await;
                        if let Some(project) = pm.active_project_mut() {
                            project.add_document(doc_id);
                        }
                        // pm.save_project() will be called after this async block
                    }
                } else {
                    tracing::warn!("Active document id {} not found in DocumentManager", doc_id);
                }
            } else if let Some(content) = editor_content {
                tracing::debug!("No active document; creating new document from editor content (len={})", content.len());
                // No active document: create one from editor content and save it
                let mut dm = document_manager.write().await;
                // Choose a title based on project or fallback
                let title = if let Some(proj_path) = &project_path_opt {
                    proj_path.file_name().and_then(|n| n.to_str()).unwrap_or("untitled").to_string()
                } else {
                    "untitled".to_string()
                };

                // Create document (Markdown assumed)
                match dm.create_document(&title, &content, cosmarium_core::document::DocumentFormat::Markdown).await {
                    Ok(new_id) => {
                        tracing::debug!("Created new document {} with title='{}'", new_id, title);
                        // Set file path if project exists
                        if let Some(proj_path) = &project_path_opt {
                            let content_dir = proj_path.join("content");
                            if let Err(e) = tokio::fs::create_dir_all(&content_dir).await {
                                tracing::error!("Failed to create content dir: {}", e);
                            } else {
                                let filename = format!("{}.{}", format!("doc_{}", new_id), cosmarium_core::document::DocumentFormat::Markdown.extension());
                                let file_path = content_dir.join(filename);
                                tracing::debug!("Setting file path for new doc {} -> {:?}", new_id, file_path);
                                if let Some(doc_mut) = dm.get_document_mut(new_id) {
                                    doc_mut.set_file_path(&file_path);
                                }
                            }
                        } else {
                            tracing::warn!("No active project path; new document {} will have no file path", new_id);
                        }

                        if let Err(e) = dm.save_document(new_id).await {
                            tracing::error!("Failed to save new document {}: {}", new_id, e);
                        } else {
                            tracing::info!("Saved new document {}", new_id);
                            // Update active document id so UI reflects saved doc
                            self.active_document_id = Some(new_id);

                            // Ensure project references this document id before saving project
                            let mut pm = project_manager.write().await;
                            if let Some(project) = pm.active_project_mut() {
                                project.add_document(new_id);
                            }
                        }
                    }
                    Err(e) => tracing::error!("Failed to create document from editor content: {}", e),
                }
            } else {
                tracing::debug!("No active document and no editor content to save");
            }

            // Finally, save project metadata
            let mut pm = project_manager.write().await;
            pm.save_project().await
        })
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
            // Try to find a content file in the project's content directory and open it.
            let pm_read = pm_clone.read().await;
            if let Some(project) = pm_read.active_project() {
                let content_dir = project.path().join("content");
                drop(pm_read); // release read lock before async file ops / write locks

                // Look for a file in content_dir with a supported extension
                let mut found: Option<std::path::PathBuf> = None;
                if let Ok(mut rd) = tokio::fs::read_dir(&content_dir).await {
                    while let Ok(Some(entry)) = rd.next_entry().await {
                        if let Ok(ft) = entry.file_type().await {
                            if ft.is_file() {
                                let path = entry.path();
                                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                                    match ext {
                                        "md" | "markdown" | "txt" | "rtf" | "html" | "htm" => {
                                            found = Some(path);
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(path) = found {
                    // Open document in DocumentManager so it becomes available in-memory
                    let mut dm_write = document_manager.write().await;
                    match dm_write.open_document(&path).await {
                        Ok(new_id) => {
                            if let Some(doc) = dm_write.get_document(new_id) {
                                return (Some(new_id), Some(doc.content().to_string()));
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to open document from path {:?}: {}", path, e)
                        }
                    }
                }
            }

            (None, None)
        });

        // Set active document and content in editor if we got any
        self.active_document_id = doc_id_opt;
        if let Some(content) = doc_content {
            self.plugin_context
                .set_shared_state("markdown_editor_content", content.clone());
            // Also write plugin-specific data as a fallback synchronization channel
            self.plugin_context.set_plugin_data(
                "markdown-editor",
                "loaded_content",
                content.clone(),
            );
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
        self.session
            .add_recent_project(path.clone(), self.config.app.max_recent_projects);
        self.session.last_opened_project = Some(path);
        if let Err(e) = self.session.save() {
            tracing::warn!("Failed to save session: {}", e);
        }

        // Update UI list
        self.recent_projects = self.session.recent_projects.clone();

        // Request focus for the editor
        self.plugin_context
            .set_shared_state("markdown_editor_focus_requested", true);

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
        self.session
            .add_recent_project(project_path.clone(), self.config.app.max_recent_projects);
        self.session.last_opened_project = Some(project_path);
        if let Err(e) = self.session.save() {
            tracing::warn!("Failed to save session: {}", e);
        }
        self.recent_projects = self.session.recent_projects.clone();

        // Request focus for the editor
        self.plugin_context
            .set_shared_state("markdown_editor_focus_requested", true);

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
                let project_name = self
                    .current_project
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
                                let label = path
                                    .file_name()
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
                let mut render_menu_item =
                    |ui: &mut egui::Ui,
                     id: MenuId,
                     label: &str,
                     content: Box<dyn FnOnce(&mut Self, &mut egui::Ui)>| {
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

                            egui::popup_below_widget(
                                ui,
                                popup_id,
                                &button_response,
                                egui::PopupCloseBehavior::CloseOnClickOutside,
                                |ui| {
                                    ui.set_min_width(150.0);
                                    ui.with_layout(
                                        egui::Layout::top_down_justified(egui::Align::Min),
                                        |ui| {
                                            content(self, ui);
                                        },
                                    );
                                },
                            );
                        }
                    };

                // Cosmarium Menu
                render_menu_item(
                    ui,
                    MenuId::Cosmarium,
                    "Cosmarium",
                    Box::new(|app, ui| {
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
                        if ui
                            .add(
                                egui::Button::new("Exit")
                                    .shortcut_text(egui::RichText::new("Ctrl+Q").size(12.0).weak()),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }),
                );

                // File Menu
                render_menu_item(
                    ui,
                    MenuId::File,
                    "File",
                    Box::new(|app, ui| {
                        if ui
                            .add(
                                egui::Button::new("New Project")
                                    .shortcut_text(egui::RichText::new("Ctrl+N").size(12.0).weak()),
                            )
                            .clicked()
                        {
                            app.show_new_project_dialog = true;
                            app.ui_state.active_menu = None;
                            app.ui_state.menu_expanded = false;
                        }
                        if ui
                            .add(
                                egui::Button::new("Open Project")
                                    .shortcut_text(egui::RichText::new("Ctrl+O").size(12.0).weak()),
                            )
                            .clicked()
                        {
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

                        if ui
                            .add(
                                egui::Button::new("Save Project")
                                    .shortcut_text(egui::RichText::new("Ctrl+S").size(12.0).weak()),
                            )
                            .clicked()
                        {
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
                    }),
                );

                // Edit Menu
                render_menu_item(
                    ui,
                    MenuId::Edit,
                    "Edit",
                    Box::new(|app, ui| {
                        if ui
                            .add(
                                egui::Button::new("Undo")
                                    .shortcut_text(egui::RichText::new("Ctrl+Z").size(12.0).weak()),
                            )
                            .clicked()
                        {
                            app.ui_state.active_menu = None;
                            app.ui_state.menu_expanded = false;
                        }
                        if ui
                            .add(
                                egui::Button::new("Redo")
                                    .shortcut_text(egui::RichText::new("Ctrl+Y").size(12.0).weak()),
                            )
                            .clicked()
                        {
                            app.ui_state.active_menu = None;
                            app.ui_state.menu_expanded = false;
                        }
                        ui.separator();
                        if ui.button("Cut").clicked() {
                            app.ui_state.active_menu = None;
                            app.ui_state.menu_expanded = false;
                        }
                        if ui.button("Copy").clicked() {
                            app.ui_state.active_menu = None;
                            app.ui_state.menu_expanded = false;
                        }
                        if ui.button("Paste").clicked() {
                            app.ui_state.active_menu = None;
                            app.ui_state.menu_expanded = false;
                        }
                    }),
                );

                // View Menu
                render_menu_item(
                    ui,
                    MenuId::View,
                    "View",
                    Box::new(|app, ui| {
                        // We need to collect changes to avoid borrowing issues
                        let mut panels_to_toggle = Vec::new();

                        for (panel_name, panel_plugin) in &app.panel_plugins {
                            if !panel_plugin.is_closable() {
                                continue;
                            }
                            let is_open =
                                app.ui_state.open_panels.get(panel_name).unwrap_or(&false);
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

                        if ui
                            .checkbox(&mut app.ui_state.show_menu_bar, "Menu Bar")
                            .clicked()
                        {
                            // app.ui_state.active_menu = None; // Optional
                        }
                        if ui
                            .checkbox(&mut app.ui_state.show_status_bar, "Status Bar")
                            .clicked()
                        {
                            // app.ui_state.active_menu = None; // Optional
                        }
                    }),
                );

                // Tools Menu
                render_menu_item(
                    ui,
                    MenuId::Tools,
                    "Tools",
                    Box::new(|app, ui| {
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
                    }),
                );

                // Help Menu
                render_menu_item(
                    ui,
                    MenuId::Help,
                    "Help",
                    Box::new(|app, ui| {
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
                    }),
                );
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
                    ui.label(format!(
                        "📁 {}",
                        project.file_stem().unwrap_or_default().to_string_lossy()
                    ));
                    ui.separator();
                } else {
                    ui.label("📝 No project");
                    ui.separator();
                }

                // Editor stats (if available from shared state)
                if let Some(word_count) = self
                    .plugin_context
                    .get_shared_state::<usize>("editor_word_count")
                {
                    ui.label(format!("Words: {}", word_count));
                    ui.separator();
                }
                if let Some(char_count) = self
                    .plugin_context
                    .get_shared_state::<usize>("editor_char_count")
                {
                    ui.label(format!("Characters: {}", char_count));
                    ui.separator();
                }
                if let Some(para_count) = self
                    .plugin_context
                    .get_shared_state::<usize>("editor_para_count")
                {
                    ui.label(format!("Paragraphs: {}", para_count));
                    ui.separator();
                }

                // Plugin status
                ui.label(format!(
                    "🔌 {} plugins",
                    self.panel_plugins.len() + self.plugins.len()
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Application info
                    ui.label(format!("Cosmarium v{}", env!("CARGO_PKG_VERSION")));
                    ui.separator();

                    // Atmosphere Status
                    let sentiment = self.plugin_context.get_shared_state::<f32>("atmosphere_sentiment").unwrap_or(0.0);

                    let is_analyzing = self.plugin_context.get_shared_state::<bool>("atmosphere_analyzing").unwrap_or(false);
                    let emotions = self.plugin_context.get_shared_state::<Vec<(String, f32)>>("atmosphere_emotions").unwrap_or_default();
                    let p_idx = self.plugin_context.get_shared_state::<usize>("atmosphere_paragraph_idx").unwrap_or(0);
                    let emotion_name = self.plugin_context.get_shared_state::<String>("atmosphere_current_emotion").unwrap_or_else(|| "Neutral".to_string());

                    if p_idx > 0 {
                        ui.label(format!("Atmo: P{}", p_idx));
                    }

                    if is_analyzing {
                        ui.add(egui::Spinner::new().size(12.0));
                    }

                    // Color square using the current theme background
                    let color = ui.visuals().panel_fill;

                    let (rect, response) = ui.allocate_at_least(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color);
                    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(128)), egui::StrokeKind::Outside);

                    if !emotions.is_empty() || p_idx > 0 {
                        response.on_hover_ui(|ui| {
                            ui.label(format!("Climat: {}", emotion_name));
                            ui.label(format!("Sentiment: {:.2}", sentiment));
                            if p_idx > 0 {
                                ui.label(format!("Paragraphe: #{}", p_idx));
                            }
                            if !emotions.is_empty() {
                                ui.separator();
                                for (emotion, score) in emotions.iter().take(3) {
                                    ui.label(format!("{}: {:.1}%", emotion, score * 100.0));
                                }
                            }
                        });
                    }
                });
            });
        });
    }

    /// Render panels based on their position.
    fn render_panels(&mut self, ctx: &egui::Context) {
        // Left panel
        if self.has_panels_in_position(cosmarium_plugin_api::PanelPosition::Left) {
            let mut frame = egui::Frame::side_top_panel(&ctx.style());
            frame.inner_margin.bottom = 0;

            egui::SidePanel::left("left_panel")
                .width_range(200.0..=400.0)
                .default_width(self.ui_state.left_panel_width)
                .frame(frame)
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
            if plugin.default_position() != position {
                return false;
            }

            // For Left panel (Zed-style), we always show the container if plugins exist,
            // so the tab bar is visible.
            if position == cosmarium_plugin_api::PanelPosition::Left {
                return true;
            }

            !plugin.is_closable() || *self.ui_state.open_panels.get(name).unwrap_or(&false)
        })
    }

    /// Render all panels in the specified position.
    fn render_panels_in_position(
        &mut self,
        ui: &mut egui::Ui,
        position: cosmarium_plugin_api::PanelPosition,
    ) {
        let panels_to_render: Vec<String> = self
            .panel_plugins
            .iter()
            .filter_map(|(name, plugin)| {
                if plugin.default_position() != position {
                    return None;
                }

                // For Left panel (Zed-style), we include all plugins in the list
                if position == cosmarium_plugin_api::PanelPosition::Left {
                    return Some(name.clone());
                }

                if !plugin.is_closable() || *self.ui_state.open_panels.get(name).unwrap_or(&false) {
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

        // Special handling for Left panel (Zed-style tabs)
        if position == cosmarium_plugin_api::PanelPosition::Left {
            if panels_to_render.is_empty() {
                return;
            }

            // Ensure we have an active panel
            if self.ui_state.active_left_panel.is_none()
                || !panels_to_render.contains(self.ui_state.active_left_panel.as_ref().unwrap())
            {
                self.ui_state.active_left_panel = Some(panels_to_render[0].clone());
            }

            let active_panel_name = self.ui_state.active_left_panel.clone().unwrap();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                // Add spacing at the bottom to avoid overlapping the status bar
                ui.add_space(10.0);

                // Render tab bar at the bottom
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; // Compact tabs
                    for panel_name in &panels_to_render {
                        if let Some(panel) = self.panel_plugins.get(panel_name) {
                            let icon = panel.panel_icon();
                            let is_active = *panel_name == active_panel_name;

                            // Style the tab button
                            let text = egui::RichText::new(icon).size(16.0).color(if is_active {
                                ui.visuals().text_color()
                            } else {
                                ui.visuals().weak_text_color()
                            });

                            let response = ui.add(
                                egui::Button::new(text)
                                    .frame(false)
                                    .min_size(egui::vec2(40.0, 30.0)),
                            );

                            if response.clicked() {
                                self.ui_state.active_left_panel = Some(panel_name.clone());
                            }

                            // Tooltip with title
                            if response.hovered() {
                                response.on_hover_text(panel.panel_title());
                            }
                        }
                    }
                });
                ui.separator();

                // Render active panel content in remaining space
                if let Some(panel) = self.panel_plugins.get_mut(&active_panel_name) {
                    // We want the panel to fill the space
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        panel.render_panel(ui, &mut self.plugin_context);
                    });
                }
            });
        } else {
            // Standard stacking behavior for other positions
            for panel_name in &panels_to_render {
                if let Some(panel) = self.panel_plugins.get_mut(panel_name) {
                    if !panel.is_closable() {
                        // Render directly without header for non-closable panels (like Editor)
                        panel.render_panel(ui, &mut self.plugin_context);
                    } else {
                        // Create a collapsing header for each panel
                        let title = panel.panel_title().to_string();
                        let header_response = ui.collapsing(title, |ui| {
                            panel.render_panel(ui, &mut self.plugin_context);
                        });

                        // Handle panel closing if closable
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
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let is_open =
                                        self.ui_state.open_panels.get(name).unwrap_or(&false);
                                    if *is_open {
                                        ui.colored_label(egui::Color32::GREEN, "Active");
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "Inactive");
                                    }
                                },
                            );
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
                                ui.selectable_value(
                                    &mut self.ui_state.current_theme,
                                    "Dark".to_string(),
                                    "Dark",
                                );
                                ui.selectable_value(
                                    &mut self.ui_state.current_theme,
                                    "Light".to_string(),
                                    "Light",
                                );
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
                                ui.selectable_value(
                                    &mut self.new_project_template,
                                    "novel".to_string(),
                                    "Novel",
                                );
                                ui.selectable_value(
                                    &mut self.new_project_template,
                                    "short-story".to_string(),
                                    "Short Story",
                                );
                                ui.selectable_value(
                                    &mut self.new_project_template,
                                    "screenplay".to_string(),
                                    "Screenplay",
                                );
                                ui.selectable_value(
                                    &mut self.new_project_template,
                                    "blog".to_string(),
                                    "Blog",
                                );
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
        // Handle close request
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.handle_close_request() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // Update plugins
        for plugin in self.plugins.values_mut() {
            if let Err(e) = plugin.update(&mut self.plugin_context) {
                tracing::error!("Plugin update error: {}", e);
            }
        }

        // Update panel plugins
        for plugin in self.panel_plugins.values_mut() {
            if let Err(e) = plugin.update(&mut self.plugin_context) {
                tracing::error!("Panel plugin update error: {}", e);
            }
        }

        // Update atmosphere
        self.update_atmosphere(ctx);

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
                        self.plugin_context
                            .set_shared_state("markdown_editor_action", "redo".to_string());
                    } else {
                        // Undo (Ctrl+Z)
                        self.plugin_context
                            .set_shared_state("markdown_editor_action", "undo".to_string());
                    }
                } else if input.key_pressed(egui::Key::Y) {
                    // Redo (Ctrl+Y)
                    self.plugin_context
                        .set_shared_state("markdown_editor_action", "redo".to_string());
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
        self.render_close_confirmation(ctx);
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

#[derive(Deserialize, Debug, Clone)]
struct SharedPalette {
    main_bg_h: f32,
    main_bg_s: f32,
    main_bg_l: f32,
    main_fg_h: f32,
    main_fg_s: f32,
    main_fg_l: f32,
    is_light: bool,
}

impl Cosmarium {
    fn handle_close_request(&mut self) -> bool {
        // If force close is set, allow closing immediately
        if self.force_close {
            return true;
        }

        // Sync content first to ensure we have latest state
        self.sync_editor_content();

        // Check for unsaved changes
        if self.check_unsaved_changes() {
            self.show_close_confirmation = true;
            // Prevent closing, show dialog instead
            return false;
        }

        // No unsaved changes, allow closing
        true
    }

    fn render_close_confirmation(&mut self, ctx: &egui::Context) {
        if self.show_close_confirmation {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_width(300.0);
                    ui.heading("Unsaved Changes");
                    ui.label("You have unsaved changes. Do you want to save them before closing?");

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            if let Err(e) = self.save_current_project() {
                                tracing::error!("Failed to save project on close: {}", e);
                            }
                            self.show_close_confirmation = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        if ui.button("Don't Save").clicked() {
                            // Discard changes and close
                            self.show_close_confirmation = false;
                            self.force_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_close_confirmation = false;
                        }
                    });
                });
        }
    }

    /// Update the atmosphere (theme) based on sentiment and intensity.
    fn update_atmosphere(&mut self, ctx: &egui::Context) {
        // Read raw values from shared state
        let target_sentiment = self.plugin_context.get_shared_state::<f32>("atmosphere_sentiment").unwrap_or(0.0);
        let target_intensity = self.plugin_context.get_shared_state::<f32>("atmosphere_intensity").unwrap_or(0.0);
        
        // Update smoothed values (low-pass filter for stability)
        // We want to move towards target values over time
        let dt = ctx.input(|i| i.stable_dt).min(0.1); // Cap dt
        let s_speed = 4.0; // Sentiment speed
        let i_speed = 6.0; // Intensity speed (slightly snappier)

        let s_diff = target_sentiment - self.current_sentiment;
        if s_diff.abs() > 0.001 {
            self.current_sentiment += s_diff * s_speed * dt;
            ctx.request_repaint();
        } else {
            self.current_sentiment = target_sentiment;
        }

        let i_diff = target_intensity - self.current_intensity;
        if i_diff.abs() > 0.001 {
            self.current_intensity += i_diff * i_speed * dt;
            ctx.request_repaint();
        } else {
            self.current_intensity = target_intensity;
        }

        let sentiment = self.current_sentiment;
        let intensity = self.current_intensity;

        // 1. Try to get the rich RYB-based palette from shared state
        let shared_palette = self
            .plugin_context
            .get_shared_state::<String>("atmosphere_palette")
            .and_then(|json| serde_json::from_str::<SharedPalette>(&json).ok());

        // 2. Determine target colors (either from SharedPalette or HSL fallback)
        let (hue, saturation, lightness_bg, _lightness_fg) = if let Some(p) = shared_palette {
            (p.main_bg_h, p.main_bg_s / 100.0, p.main_bg_l / 100.0, p.main_fg_l / 100.0)
        } else if sentiment > 0.0 {
            // Fallback Positive: Gold/Warm (Hue ~45)
            (45.0, 0.6 * sentiment.abs(), 0.95, 0.1)
        } else {
            // Fallback Negative: Blue/Cool (Hue ~220)
            (220.0, 0.5 * sentiment.abs(), 0.05, 0.9)
        };

        // Helper to convert HSL to Color32
        fn hsl_to_color(h: f32, s: f32, l: f32) -> egui::Color32 {
            let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
            let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
            let m = l - c / 2.0;

            let (r, g, b) = if h < 60.0 {
                (c, x, 0.0)
            } else if h < 120.0 {
                (x, c, 0.0)
            } else if h < 180.0 {
                (0.0, c, x)
            } else if h < 240.0 {
                (0.0, x, c)
            } else if h < 300.0 {
                (x, 0.0, c)
            } else {
                (c, 0.0, x)
            };

            egui::Color32::from_rgb(
                ((r + m) * 255.0) as u8,
                ((g + m) * 255.0) as u8,
                ((b + m) * 255.0) as u8,
            )
        }

        // Base neutral (Dark Mode default)
        let base_bg = egui::Color32::from_rgb(27, 27, 27);

        // Target colors
        let target_bg = hsl_to_color(hue, saturation, lightness_bg);

        // Interpolate background
        // Use intensity as the primary transition factor
        let factor = intensity.clamp(0.0, 1.0).powf(0.5); // Sqrt makes 0.25 -> 0.5, 0.5 -> 0.7, etc.

        let new_bg = lerp_color(base_bg, target_bg, factor);

        // Faint background (for panels/windows)
        let new_faint = lerp_color(
            egui::Color32::from_additive_luminance(10),
            hsl_to_color(
                hue,
                saturation * 0.5,
                lightness_bg * 0.9 + (if lightness_bg > 0.5 { -0.1 } else { 0.1 }),
            ),
            factor,
        );

        // Dynamic Contrast Calculation
        let bg_lum = 0.299 * new_bg.r() as f32 + 0.587 * new_bg.g() as f32 + 0.114 * new_bg.b() as f32;

        let new_fg = if bg_lum > 140.0 {
            egui::Color32::from_rgb(10, 10, 15)
        } else {
            egui::Color32::from_rgb(240, 240, 245)
        };

        let mut visuals = if bg_lum > 140.0 {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };

        visuals.panel_fill = new_bg;
        visuals.window_fill = new_bg;
        visuals.faint_bg_color = new_faint;
        visuals.extreme_bg_color = new_bg;

        visuals.widgets.noninteractive.fg_stroke.color = new_fg;
        visuals.widgets.inactive.fg_stroke.color = new_fg;
        visuals.widgets.hovered.fg_stroke.color = new_fg;
        visuals.widgets.active.fg_stroke.color = new_fg;
        visuals.widgets.open.fg_stroke.color = new_fg;

        visuals.selection.bg_fill = hsl_to_color(hue, 0.8, 0.5);
        visuals.selection.stroke.color = new_fg;

        ctx.set_visuals(visuals);
    }
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = (a.r() as f32 * (1.0 - t) + b.r() as f32 * t) as u8;
    let g = (a.g() as f32 * (1.0 - t) + b.g() as f32 * t) as u8;
    let b_val = (a.b() as f32 * (1.0 - t) + b.b() as f32 * t) as u8;
    egui::Color32::from_rgb(r, g, b_val)
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
