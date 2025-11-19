//! # Markdown preview module for the Markdown Editor plugin
//!
//! This module provides live preview functionality for markdown content,
//! allowing writers to see how their markdown will be rendered while they write.
//! It supports HTML rendering, custom CSS styling, and synchronized scrolling.

use cosmarium_plugin_api::Result;
#[cfg(feature = "live-preview")]
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Markdown preview renderer.
///
/// The [`PreviewRenderer`] converts markdown content to HTML and provides
/// various rendering options and customizations for the preview display.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "live-preview")]
/// # {
/// use cosmarium_markdown_editor::preview::PreviewRenderer;
///
/// let renderer = PreviewRenderer::new();
/// let html = renderer.render("# Hello World").unwrap();
/// assert!(html.contains("<h1>"));
/// # }
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PreviewRenderer {
    /// Markdown parsing options
    #[cfg(feature = "live-preview")]
    options: Options,
    #[cfg(not(feature = "live-preview"))]
    options: (),
    /// Custom CSS styles
    custom_css: String,
    /// Base HTML template
    template: String,
    /// Theme name
    theme: String,
    /// Whether to enable syntax highlighting
    syntax_highlighting: bool,
    /// Custom replacements for text processing
    replacements: HashMap<String, String>,
}

impl PreviewRenderer {
    /// Create a new preview renderer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let renderer = PreviewRenderer::new();
    /// ```
    pub fn new() -> Self {
        #[cfg(feature = "live-preview")]
        let options = {
            let mut opts = Options::empty();
            opts.insert(Options::ENABLE_STRIKETHROUGH);
            opts.insert(Options::ENABLE_TABLES);
            opts.insert(Options::ENABLE_FOOTNOTES);
            opts.insert(Options::ENABLE_TASKLISTS);
            opts.insert(Options::ENABLE_SMART_PUNCTUATION);
            opts
        };
        
        #[cfg(not(feature = "live-preview"))]
        let options = Default::default();

        Self {
            options,
            custom_css: Self::default_css(),
            template: Self::default_template(),
            theme: "default".to_string(),
            syntax_highlighting: true,
            replacements: HashMap::new(),
        }
    }

    /// Render markdown content to HTML.
    ///
    /// # Arguments
    ///
    /// * `markdown` - Markdown content to render
    ///
    /// # Returns
    ///
    /// Rendered HTML content.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "live-preview")]
    /// # {
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let renderer = PreviewRenderer::new();
    /// let html = renderer.render("# Hello **World**").unwrap();
    /// assert!(html.contains("<h1>Hello <strong>World</strong></h1>"));
    /// # }
    /// ```
    pub fn render(&self, markdown: &str) -> Result<String> {
        #[cfg(feature = "live-preview")]
        {
            // Apply custom replacements
            let processed_markdown = self.apply_replacements(markdown);
            
            // Parse markdown
            let parser = Parser::new_ext(&processed_markdown, self.options);
            
            // Render to HTML
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            
            // Apply syntax highlighting if enabled
            let final_html = if self.syntax_highlighting {
                self.apply_syntax_highlighting(html_output)
            } else {
                html_output
            };
            
            // Wrap in template
            let full_html = self.template
                .replace("{content}", &final_html)
                .replace("{css}", &self.custom_css)
                .replace("{theme}", &self.theme);
            
            Ok(full_html)
        }
        
        #[cfg(not(feature = "live-preview"))]
        {
            Ok(format!("<pre>{}</pre>", markdown))
        }
    }

    /// Render markdown content to plain HTML without template wrapping.
    ///
    /// # Arguments
    ///
    /// * `markdown` - Markdown content to render
    ///
    /// # Returns
    ///
    /// Raw HTML content without CSS or template.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "live-preview")]
    /// # {
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let renderer = PreviewRenderer::new();
    /// let html = renderer.render_fragment("**bold text**").unwrap();
    /// assert_eq!(html, "<p><strong>bold text</strong></p>\n");
    /// # }
    /// ```
    pub fn render_fragment(&self, markdown: &str) -> Result<String> {
        #[cfg(feature = "live-preview")]
        {
            let processed_markdown = self.apply_replacements(markdown);
            let parser = Parser::new_ext(&processed_markdown, self.options);
            
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            
            Ok(html_output)
        }
        
        #[cfg(not(feature = "live-preview"))]
        {
            Ok(format!("<p>{}</p>", markdown))
        }
    }

    /// Set custom CSS for the preview.
    ///
    /// # Arguments
    ///
    /// * `css` - CSS content to use for styling
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let mut renderer = PreviewRenderer::new();
    /// renderer.set_custom_css("body { font-family: serif; }");
    /// ```
    pub fn set_custom_css(&mut self, css: &str) {
        self.custom_css = css.to_string();
    }

    /// Get the current custom CSS.
    pub fn custom_css(&self) -> &str {
        &self.custom_css
    }

    /// Set the preview theme.
    ///
    /// # Arguments
    ///
    /// * `theme` - Theme name to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let mut renderer = PreviewRenderer::new();
    /// renderer.set_theme("dark");
    /// ```
    pub fn set_theme(&mut self, theme: &str) {
        self.theme = theme.to_string();
        // Update CSS based on theme
        self.update_theme_css();
    }

    /// Get the current theme.
    pub fn theme(&self) -> &str {
        &self.theme
    }

    /// Enable or disable syntax highlighting.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable syntax highlighting
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let mut renderer = PreviewRenderer::new();
    /// renderer.set_syntax_highlighting(false);
    /// ```
    pub fn set_syntax_highlighting(&mut self, enabled: bool) {
        self.syntax_highlighting = enabled;
    }

    /// Check if syntax highlighting is enabled.
    pub fn syntax_highlighting(&self) -> bool {
        self.syntax_highlighting
    }

    /// Add a text replacement rule.
    ///
    /// # Arguments
    ///
    /// * `from` - Text to replace
    /// * `to` - Replacement text
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::preview::PreviewRenderer;
    ///
    /// let mut renderer = PreviewRenderer::new();
    /// renderer.add_replacement("--", "—"); // Replace double dash with em dash
    /// ```
    pub fn add_replacement(&mut self, from: &str, to: &str) {
        self.replacements.insert(from.to_string(), to.to_string());
    }

    /// Remove a text replacement rule.
    ///
    /// # Arguments
    ///
    /// * `from` - Text replacement to remove
    pub fn remove_replacement(&mut self, from: &str) {
        self.replacements.remove(from);
    }

    /// Clear all text replacement rules.
    pub fn clear_replacements(&mut self) {
        self.replacements.clear();
    }

    /// Apply text replacements to markdown content.
    #[allow(dead_code)]
    fn apply_replacements(&self, markdown: &str) -> String {
        let mut result = markdown.to_string();
        
        for (from, to) in &self.replacements {
            result = result.replace(from, to);
        }
        
        result
    }

    /// Apply syntax highlighting to HTML content.
    #[allow(dead_code)]
    fn apply_syntax_highlighting(&self, html: String) -> String {
        // This is a placeholder implementation
        // In a real implementation, you would integrate with a syntax highlighting library
        // such as syntect or highlight.js
        html
    }

    /// Update CSS based on the current theme.
    fn update_theme_css(&mut self) {
        let theme_css = match self.theme.as_str() {
            "dark" => Self::dark_theme_css(),
            "light" => Self::light_theme_css(),
            "sepia" => Self::sepia_theme_css(),
            _ => Self::default_css(),
        };
        
        self.custom_css = theme_css;
    }

    /// Get the default CSS for preview rendering.
    fn default_css() -> String {
        r#"
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 16px;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #fff;
        }
        
        h1, h2, h3, h4, h5, h6 {
            margin-top: 24px;
            margin-bottom: 16px;
            font-weight: 600;
            line-height: 1.25;
        }
        
        h1 { font-size: 2em; border-bottom: 1px solid #eee; padding-bottom: 10px; }
        h2 { font-size: 1.5em; }
        h3 { font-size: 1.25em; }
        
        p { margin-bottom: 16px; }
        
        code {
            background-color: rgba(27,31,35,0.05);
            border-radius: 3px;
            font-size: 85%;
            margin: 0;
            padding: 0.2em 0.4em;
        }
        
        pre {
            background-color: #f6f8fa;
            border-radius: 6px;
            font-size: 85%;
            line-height: 1.45;
            overflow: auto;
            padding: 16px;
        }
        
        blockquote {
            border-left: 4px solid #dfe2e5;
            margin: 0;
            padding: 0 16px;
            color: #6a737d;
        }
        
        table {
            border-collapse: collapse;
            width: 100%;
        }
        
        th, td {
            border: 1px solid #dfe2e5;
            padding: 8px 12px;
            text-align: left;
        }
        
        th {
            background-color: #f6f8fa;
            font-weight: 600;
        }
        "#.to_string()
    }

    /// Get the default HTML template.
    fn default_template() -> String {
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Markdown Preview</title>
    <style>
        {css}
    </style>
</head>
<body class="theme-{theme}">
    {content}
</body>
</html>"#.to_string()
    }

    /// Get dark theme CSS.
    fn dark_theme_css() -> String {
        r#"
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 16px;
            line-height: 1.6;
            color: #e1e4e8;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #0d1117;
        }
        
        h1, h2, h3, h4, h5, h6 {
            margin-top: 24px;
            margin-bottom: 16px;
            font-weight: 600;
            line-height: 1.25;
            color: #f0f6fc;
        }
        
        h1 { font-size: 2em; border-bottom: 1px solid #21262d; padding-bottom: 10px; }
        
        code {
            background-color: rgba(110,118,129,0.4);
            border-radius: 3px;
            color: #e1e4e8;
        }
        
        pre {
            background-color: #161b22;
            border-radius: 6px;
            color: #e1e4e8;
        }
        
        blockquote {
            border-left: 4px solid #3b434b;
            color: #8b949e;
        }
        
        th, td {
            border: 1px solid #30363d;
        }
        
        th {
            background-color: #161b22;
        }
        "#.to_string()
    }

    /// Get light theme CSS.
    fn light_theme_css() -> String {
        Self::default_css()
    }

    /// Get sepia theme CSS.
    fn sepia_theme_css() -> String {
        r#"
        body {
            font-family: Georgia, 'Times New Roman', serif;
            font-size: 16px;
            line-height: 1.6;
            color: #5c4b37;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f4f1ea;
        }
        
        h1, h2, h3, h4, h5, h6 {
            color: #704214;
        }
        
        code {
            background-color: #ede0c8;
            color: #5c4b37;
        }
        
        pre {
            background-color: #ede0c8;
            color: #5c4b37;
        }
        
        blockquote {
            border-left: 4px solid #d4c5a9;
            color: #8b7355;
        }
        "#.to_string()
    }
}

impl Default for PreviewRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Preview settings for customizing the rendering behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewSettings {
    /// Whether to enable live preview
    pub enabled: bool,
    /// Auto-scroll to cursor position
    pub auto_scroll: bool,
    /// Sync scroll between editor and preview
    pub sync_scroll: bool,
    /// Update delay in milliseconds
    pub update_delay: u64,
    /// Custom CSS file path
    pub custom_css_file: Option<String>,
    /// MathJax support
    pub enable_math: bool,
    /// Mermaid diagram support
    pub enable_diagrams: bool,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_scroll: true,
            sync_scroll: true,
            update_delay: 500,
            custom_css_file: None,
            enable_math: false,
            enable_diagrams: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_renderer_creation() {
        let renderer = PreviewRenderer::new();
        assert_eq!(renderer.theme(), "default");
        assert!(renderer.syntax_highlighting());
    }

    #[test]
    #[cfg(feature = "live-preview")]
    fn test_basic_markdown_rendering() {
        let renderer = PreviewRenderer::new();
        let html = renderer.render_fragment("# Hello World").unwrap();
        assert!(html.contains("<h1>Hello World</h1>"));
    }

    #[test]
    #[cfg(feature = "live-preview")]
    fn test_bold_and_italic() {
        let renderer = PreviewRenderer::new();
        let html = renderer.render_fragment("**bold** and *italic*").unwrap();
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    #[cfg(feature = "live-preview")]
    fn test_code_blocks() {
        let renderer = PreviewRenderer::new();
        let html = renderer.render_fragment("```rust\nfn main() {}\n```").unwrap();
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code"));
    }

    #[test]
    fn test_theme_switching() {
        let mut renderer = PreviewRenderer::new();
        renderer.set_theme("dark");
        assert_eq!(renderer.theme(), "dark");
        
        renderer.set_theme("light");
        assert_eq!(renderer.theme(), "light");
    }

    #[test]
    #[cfg(feature = "live-preview")]
    fn test_text_replacements() {
        let mut renderer = PreviewRenderer::new();
        renderer.add_replacement("--", "—");
        
        let html = renderer.render_fragment("Hello -- world").unwrap();
        assert!(html.contains("Hello — world"));
    }

    #[test]
    fn test_custom_css() {
        let mut renderer = PreviewRenderer::new();
        renderer.set_custom_css("body { color: red; }");
        assert_eq!(renderer.custom_css(), "body { color: red; }");
    }

    #[test]
    fn test_syntax_highlighting_toggle() {
        let mut renderer = PreviewRenderer::new();
        assert!(renderer.syntax_highlighting());
        
        renderer.set_syntax_highlighting(false);
        assert!(!renderer.syntax_highlighting());
    }

    #[test]
    #[cfg(feature = "live-preview")]
    fn test_full_html_rendering() {
        let renderer = PreviewRenderer::new();
        let html = renderer.render("# Test").unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<body"));
        assert!(html.contains("Test"));
    }

    #[test]
    #[cfg(feature = "live-preview")]
    fn test_replacement_removal() {
        let mut renderer = PreviewRenderer::new();
        renderer.add_replacement("test", "replacement");
        renderer.remove_replacement("test");
        
        let html = renderer.render_fragment("test").unwrap();
        assert!(html.contains("test"));
        assert!(!html.contains("replacement"));
    }
}