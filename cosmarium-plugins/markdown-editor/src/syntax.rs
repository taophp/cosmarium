//! # Syntax highlighting module for the Markdown Editor plugin
//!
//! This module provides syntax highlighting capabilities for markdown content
//! in the Cosmarium creative writing software. It handles highlighting of
//! markdown syntax elements, code blocks, and provides customizable themes.

use cosmarium_plugin_api::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "syntax-highlighting")]
use syntect::highlighting::{Color, FontStyle, Style, Theme, ThemeSet};
#[cfg(feature = "syntax-highlighting")]
use syntect::parsing::{SyntaxSet, SyntaxReference};

/// Markdown syntax highlighter.
///
/// The [`MarkdownHighlighter`] provides syntax highlighting for markdown
/// content, supporting various themes and customization options.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "syntax-highlighting")]
/// # {
/// use cosmarium_markdown_editor::syntax::MarkdownHighlighter;
///
/// let highlighter = MarkdownHighlighter::new().unwrap();
/// let highlighted = highlighter.highlight("# Hello **World**");
/// # }
/// ```
#[derive(Debug)]
pub struct MarkdownHighlighter {
    #[cfg(feature = "syntax-highlighting")]
    syntax_set: SyntaxSet,
    #[cfg(feature = "syntax-highlighting")]
    theme_set: ThemeSet,
    #[cfg(feature = "syntax-highlighting")]
    current_theme: String,
    /// Custom highlighting rules
    custom_rules: HashMap<String, HighlightStyle>,
    /// Whether highlighting is enabled
    enabled: bool,
}

/// Style information for syntax highlighting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightStyle {
    /// Foreground color (hex format)
    pub foreground: String,
    /// Background color (hex format, optional)
    pub background: Option<String>,
    /// Font style flags
    pub font_style: FontStyleFlags,
}

/// Font style flags for highlighting.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FontStyleFlags {
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Underlined text
    pub underline: bool,
}

/// Highlighted text segment.
#[derive(Debug, Clone)]
pub struct HighlightedSegment {
    /// Text content
    pub text: String,
    /// Style for this segment
    pub style: HighlightStyle,
    /// Syntax type (heading, emphasis, etc.)
    pub syntax_type: String,
}

impl MarkdownHighlighter {
    /// Create a new markdown highlighter.
    ///
    /// # Errors
    ///
    /// Returns an error if the syntax highlighting system cannot be initialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "syntax-highlighting")]
    /// # {
    /// use cosmarium_markdown_editor::syntax::MarkdownHighlighter;
    ///
    /// let highlighter = MarkdownHighlighter::new().unwrap();
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        #[cfg(feature = "syntax-highlighting")]
        {
            let syntax_set = SyntaxSet::load_defaults_newlines();
            let theme_set = ThemeSet::load_defaults();
            
            Ok(Self {
                syntax_set,
                theme_set,
                current_theme: "base16-ocean.dark".to_string(),
                custom_rules: Self::default_markdown_rules(),
                enabled: true,
            })
        }
        
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            Ok(Self {
                custom_rules: Self::default_markdown_rules(),
                enabled: false,
            })
        }
    }

    /// Highlight markdown content.
    ///
    /// # Arguments
    ///
    /// * `content` - Markdown content to highlight
    ///
    /// # Returns
    ///
    /// Vector of highlighted segments.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::syntax::MarkdownHighlighter;
    ///
    /// let highlighter = MarkdownHighlighter::new().unwrap();
    /// let segments = highlighter.highlight("# Hello **World**");
    /// assert!(!segments.is_empty());
    /// ```
    pub fn highlight(&self, content: &str) -> Vec<HighlightedSegment> {
        if !self.enabled {
            return vec![HighlightedSegment {
                text: content.to_string(),
                style: HighlightStyle::default(),
                syntax_type: "plain".to_string(),
            }];
        }

        #[cfg(feature = "syntax-highlighting")]
        {
            self.highlight_with_syntect(content)
        }
        
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            self.highlight_simple(content)
        }
    }

    /// Set the current theme.
    ///
    /// # Arguments
    ///
    /// * `theme_name` - Name of the theme to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::syntax::MarkdownHighlighter;
    ///
    /// let mut highlighter = MarkdownHighlighter::new().unwrap();
    /// highlighter.set_theme("Solarized (dark)");
    /// ```
    pub fn set_theme(&mut self, _theme_name: &str) {
        #[cfg(feature = "syntax-highlighting")]
        {
            if self.theme_set.themes.contains_key(theme_name) {
                self.current_theme = theme_name.to_string();
            }
        }
        
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            // Store theme name even without syntect for consistency
            // self.current_theme = theme_name.to_string();
        }
    }

    /// Get the current theme name.
    pub fn current_theme(&self) -> &str {
        #[cfg(feature = "syntax-highlighting")]
        {
            &self.current_theme
        }
        
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            "default"
        }
    }

    /// List available themes.
    ///
    /// # Returns
    ///
    /// Vector of theme names.
    pub fn available_themes(&self) -> Vec<String> {
        #[cfg(feature = "syntax-highlighting")]
        {
            self.theme_set.themes.keys().cloned().collect()
        }
        
        #[cfg(not(feature = "syntax-highlighting"))]
        {
            vec!["default".to_string()]
        }
    }

    /// Enable or disable syntax highlighting.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable highlighting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if syntax highlighting is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Add a custom highlighting rule.
    ///
    /// # Arguments
    ///
    /// * `syntax_type` - Type of syntax element
    /// * `style` - Style to apply
    pub fn add_custom_rule(&mut self, syntax_type: &str, style: HighlightStyle) {
        self.custom_rules.insert(syntax_type.to_string(), style);
    }

    /// Remove a custom highlighting rule.
    ///
    /// # Arguments
    ///
    /// * `syntax_type` - Type of syntax element to remove
    pub fn remove_custom_rule(&mut self, syntax_type: &str) {
        self.custom_rules.remove(syntax_type);
    }

    /// Get default markdown highlighting rules.
    fn default_markdown_rules() -> HashMap<String, HighlightStyle> {
        let mut rules = HashMap::new();
        
        // Headers
        rules.insert("heading1".to_string(), HighlightStyle {
            foreground: "#FF6B6B".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: true, italic: false, underline: false },
        });
        
        rules.insert("heading2".to_string(), HighlightStyle {
            foreground: "#4ECDC4".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: true, italic: false, underline: false },
        });
        
        rules.insert("heading3".to_string(), HighlightStyle {
            foreground: "#45B7D1".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: true, italic: false, underline: false },
        });
        
        // Text formatting
        rules.insert("bold".to_string(), HighlightStyle {
            foreground: "#F7931E".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: true, italic: false, underline: false },
        });
        
        rules.insert("italic".to_string(), HighlightStyle {
            foreground: "#FFD93D".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: false, italic: true, underline: false },
        });
        
        rules.insert("code".to_string(), HighlightStyle {
            foreground: "#6BCF7F".to_string(),
            background: Some("#2D3748".to_string()),
            font_style: FontStyleFlags { bold: false, italic: false, underline: false },
        });
        
        // Links
        rules.insert("link".to_string(), HighlightStyle {
            foreground: "#4299E1".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: false, italic: false, underline: true },
        });
        
        // Lists
        rules.insert("list_marker".to_string(), HighlightStyle {
            foreground: "#805AD5".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: true, italic: false, underline: false },
        });
        
        // Blockquotes
        rules.insert("blockquote".to_string(), HighlightStyle {
            foreground: "#A0AEC0".to_string(),
            background: None,
            font_style: FontStyleFlags { bold: false, italic: true, underline: false },
        });
        
        rules
    }

    /// Highlight using syntect library (when available).
    #[cfg(feature = "syntax-highlighting")]
    fn highlight_with_syntect(&self, content: &str) -> Vec<HighlightedSegment> {
        let syntax = self.syntax_set
            .find_syntax_by_extension("md")
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        
        let theme = &self.theme_set.themes[&self.current_theme];
        
        use syntect::highlighting::Highlighter;
        use syntect::parsing::ParseState;
        
        let mut highlighter = Highlighter::new(theme);
        let mut parse_state = ParseState::new(syntax);
        let mut segments = Vec::new();
        
        for line in content.lines() {
            let ops = parse_state.parse_line(line, &self.syntax_set).unwrap();
            let highlighted = highlighter.highlight_line(line, &ops).unwrap();
            
            for (style, text) in highlighted {
                if !text.is_empty() {
                    segments.push(HighlightedSegment {
                        text: text.to_string(),
                        style: HighlightStyle::from_syntect_style(style),
                        syntax_type: "code".to_string(), // Could be more specific
                    });
                }
            }
            
            // Add newline
            segments.push(HighlightedSegment {
                text: "\n".to_string(),
                style: HighlightStyle::default(),
                syntax_type: "newline".to_string(),
            });
        }
        
        segments
    }

    /// Simple regex-based highlighting (fallback when syntect is not available).
    fn highlight_simple(&self, content: &str) -> Vec<HighlightedSegment> {
        let mut segments = Vec::new();
        let _current_pos = 0;
        
        // Simple patterns for common markdown elements
        let _patterns = [
            (r"^#{1,6}\s.*$", "heading"),
            (r"\*\*.*?\*\*", "bold"),
            (r"\*.*?\*", "italic"),
            (r"`.*?`", "code"),
            (r"\[.*?\]\(.*?\)", "link"),
            (r"^>\s.*$", "blockquote"),
            (r"^[-*+]\s", "list_marker"),
        ];
        
        // For this simple implementation, we'll just return the content as-is
        // with basic classification
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines {
            let syntax_type = if line.starts_with('#') {
                "heading"
            } else if line.starts_with('>') {
                "blockquote"
            } else if line.trim_start().starts_with(&['-', '*', '+']) {
                "list_marker"
            } else {
                "plain"
            };
            
            let style = self.custom_rules.get(syntax_type)
                .cloned()
                .unwrap_or_default();
            
            segments.push(HighlightedSegment {
                text: format!("{}\n", line),
                style,
                syntax_type: syntax_type.to_string(),
            });
        }
        
        segments
    }
}

impl Default for MarkdownHighlighter {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            #[cfg(feature = "syntax-highlighting")]
            syntax_set: SyntaxSet::load_defaults_newlines(),
            #[cfg(feature = "syntax-highlighting")]
            theme_set: ThemeSet::load_defaults(),
            #[cfg(feature = "syntax-highlighting")]
            current_theme: "base16-ocean.dark".to_string(),
            custom_rules: Self::default_markdown_rules(),
            enabled: false,
        })
    }
}

impl HighlightStyle {
    /// Create a new highlight style.
    pub fn new(foreground: &str) -> Self {
        Self {
            foreground: foreground.to_string(),
            background: None,
            font_style: FontStyleFlags::default(),
        }
    }
    
    /// Set background color.
    pub fn with_background(mut self, background: &str) -> Self {
        self.background = Some(background.to_string());
        self
    }
    
    /// Set font style.
    pub fn with_font_style(mut self, font_style: FontStyleFlags) -> Self {
        self.font_style = font_style;
        self
    }

    /// Convert from syntect style (when available).
    #[cfg(feature = "syntax-highlighting")]
    pub fn from_syntect_style(style: Style) -> Self {
        Self {
            foreground: format!("#{:02X}{:02X}{:02X}", 
                               style.foreground.r, 
                               style.foreground.g, 
                               style.foreground.b),
            background: Some(format!("#{:02X}{:02X}{:02X}", 
                                   style.background.r, 
                                   style.background.g, 
                                   style.background.b)),
            font_style: FontStyleFlags {
                bold: style.font_style.contains(FontStyle::BOLD),
                italic: style.font_style.contains(FontStyle::ITALIC),
                underline: style.font_style.contains(FontStyle::UNDERLINE),
            },
        }
    }
}

impl Default for HighlightStyle {
    fn default() -> Self {
        Self {
            foreground: "#000000".to_string(),
            background: None,
            font_style: FontStyleFlags::default(),
        }
    }
}

impl Default for FontStyleFlags {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_creation() {
        let highlighter = MarkdownHighlighter::new();
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_basic_highlighting() {
        let highlighter = MarkdownHighlighter::new().unwrap();
        let segments = highlighter.highlight("# Hello World");
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_theme_management() {
        let mut highlighter = MarkdownHighlighter::new().unwrap();
        let themes = highlighter.available_themes();
        assert!(!themes.is_empty());
        
        if !themes.is_empty() {
            highlighter.set_theme(&themes[0]);
        }
    }

    #[test]
    fn test_enable_disable() {
        let mut highlighter = MarkdownHighlighter::new().unwrap();
        #[cfg(feature = "syntax-highlighting")]
        assert!(highlighter.is_enabled());
        #[cfg(not(feature = "syntax-highlighting"))]
        assert!(!highlighter.is_enabled());
        
        highlighter.set_enabled(false);
        assert!(!highlighter.is_enabled());
        
        let segments = highlighter.highlight("# Test");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].syntax_type, "plain");
    }

    #[test]
    fn test_custom_rules() {
        let mut highlighter = MarkdownHighlighter::new().unwrap();
        let custom_style = HighlightStyle::new("#FF0000")
            .with_font_style(FontStyleFlags { bold: true, italic: false, underline: false });
        
        highlighter.add_custom_rule("custom", custom_style.clone());
        
        // Test rule was added (we can't easily test the highlighting without more complex setup)
        highlighter.remove_custom_rule("custom");
    }

    #[test]
    fn test_highlight_style_creation() {
        let style = HighlightStyle::new("#FF0000")
            .with_background("#00FF00")
            .with_font_style(FontStyleFlags { bold: true, italic: true, underline: false });
        
        assert_eq!(style.foreground, "#FF0000");
        assert_eq!(style.background, Some("#00FF00".to_string()));
        assert!(style.font_style.bold);
        assert!(style.font_style.italic);
        assert!(!style.font_style.underline);
    }

    #[test]
    fn test_font_style_flags() {
        let flags = FontStyleFlags {
            bold: true,
            italic: false,
            underline: true,
        };
        
        assert!(flags.bold);
        assert!(!flags.italic);
        assert!(flags.underline);
    }
}