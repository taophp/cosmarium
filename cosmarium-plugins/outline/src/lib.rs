use cosmarium_plugin_api::{
    PanelPlugin, PanelPosition, Plugin, PluginContext, PluginInfo, PluginType, Result,
};
use egui::Ui;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::HashSet;

pub struct OutlinePlugin {
    /// Cached headers: (level, text, line_number)
    headers: Vec<(u32, String, usize)>,
    /// Last content hash to detect changes
    last_content_hash: u64,
    /// Manually expanded nodes (by header index or similar stable ID)
    expanded_nodes: HashSet<usize>,
    /// Current active header index (based on cursor)
    active_header_index: Option<usize>,
}

impl Default for OutlinePlugin {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            last_content_hash: 0,
            expanded_nodes: HashSet::new(),
            active_header_index: None,
        }
    }
}

impl OutlinePlugin {
    pub fn new() -> Self {
        Self::default()
    }

    fn parse_headers(&mut self, content: &str) {
        self.headers.clear();

        // We need line numbers. pulldown-cmark provides byte offsets.
        // We can build a line index map.
        let line_starts: Vec<usize> = std::iter::once(0)
            .chain(content.match_indices('\n').map(|(i, _)| i + 1))
            .collect();

        let parser = Parser::new_ext(content, Options::empty()).into_offset_iter();

        let mut current_header_level = None;
        let mut current_header_text = String::new();
        let mut current_header_start = 0;

        for (event, range) in parser {
            match event {
                Event::Start(Tag::Heading(level, _, _)) => {
                    current_header_level = Some(level as u32);
                    current_header_text.clear();
                    current_header_start = range.start;
                }
                Event::Text(text) => {
                    if current_header_level.is_some() {
                        current_header_text.push_str(&text);
                    }
                }
                Event::End(Tag::Heading(_, _, _)) => {
                    if let Some(level) = current_header_level {
                        // Find line number
                        let line_number = match line_starts.binary_search(&current_header_start) {
                            Ok(idx) => idx,
                            Err(idx) => idx.saturating_sub(1),
                        };

                        self.headers
                            .push((level, current_header_text.clone(), line_number + 1));
                    }
                    current_header_level = None;
                }
                _ => {}
            }
        }

        // Update hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        self.last_content_hash = hasher.finish();
    }
}

impl Plugin for OutlinePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new(
            "outline",
            "0.1.0",
            "Document outline view",
            "Cosmarium Team",
        )
        .with_dependency("markdown-editor")
    }

    fn initialize(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Panel
    }

    fn update(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
}

impl PanelPlugin for OutlinePlugin {
    fn panel_title(&self) -> &str {
        "Outline"
    }

    fn panel_icon(&self) -> &str {
        "📑"
    }

    fn default_position(&self) -> PanelPosition {
        PanelPosition::Left
    }

    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Check for content updates
        if let Some(content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            // tracing::info!("Outline received content update, length: {}", content.len());
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            use std::hash::{Hash, Hasher};
            content.hash(&mut hasher);
            let new_hash = hasher.finish();

            if new_hash != self.last_content_hash {
                tracing::info!("Outline parsing new content");
                self.parse_headers(&content);
            }
        } else {
            // tracing::warn!("Outline: No content in shared state");
        }

        // Check for cursor updates
        if let Some(cursor_line) = ctx.get_shared_state::<usize>("markdown_editor_cursor_line") {
            // Find the header just before or at the cursor line
            let mut new_active = None;
            for (i, (_, _, line)) in self.headers.iter().enumerate() {
                if *line <= cursor_line {
                    new_active = Some(i);
                } else {
                    break;
                }
            }
            self.active_header_index = new_active;
        }

        Ok(())
    }

    fn render_panel(&mut self, ui: &mut Ui, ctx: &mut PluginContext) {
        if self.headers.is_empty() {
            ui.label("No headers found");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Simple indentation-based rendering for now (placeholder for egui_ltreeview)
            for (i, (level, text, line)) in self.headers.iter().enumerate() {
                let indent = (*level as f32 - 1.0) * 10.0;

                let is_active = self.active_header_index == Some(i);

                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    let label = if is_active {
                        egui::RichText::new(text)
                            .strong()
                            .color(ui.visuals().text_color())
                    } else {
                        egui::RichText::new(text)
                    };

                    if ui
                        .add(egui::Label::new(label).sense(egui::Sense::click()))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        // Navigate to line
                        ctx.set_shared_state("markdown_editor_goto_line", *line);
                        self.active_header_index = Some(i);
                    }
                });
            }
        });
    }
}
