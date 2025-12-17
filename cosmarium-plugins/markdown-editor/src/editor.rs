//! # Markdown editor core functionality
//!
//! This module provides the core markdown editing functionality for the
//! Cosmarium creative writing software. It handles text editing, syntax
//! highlighting, and editor-specific features for an optimal writing experience.

use serde::{Deserialize, Serialize};

/// Core markdown editor implementation.
///
/// The [`MarkdownEditor`] provides text editing capabilities optimized
/// for creative writing in Markdown format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownEditor {
    /// Current cursor position
    cursor_position: usize,
    /// Current selection range (start, end)
    selection: Option<(usize, usize)>,
    /// Undo history
    undo_history: Vec<String>,
    /// Redo history
    redo_history: Vec<String>,
    /// Maximum undo history size
    max_undo_history: usize,
}

impl MarkdownEditor {
    /// Create a new markdown editor instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::editor::MarkdownEditor;
    ///
    /// let editor = MarkdownEditor::new();
    /// ```
    pub fn new() -> Self {
        Self {
            cursor_position: 0,
            selection: None,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            max_undo_history: 100,
        }
    }

    /// Get the current cursor position.
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// Set the cursor position.
    pub fn set_cursor_position(&mut self, position: usize) {
        self.cursor_position = position;
    }

    /// Get the current selection range.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection
    }

    /// Set the selection range.
    pub fn set_selection(&mut self, start: usize, end: usize) {
        self.selection = Some((start, end));
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Add a state to the undo history.
    pub fn add_to_history(&mut self, content: String) {
        self.undo_history.push(content);

        // Limit history size
        if self.undo_history.len() > self.max_undo_history {
            self.undo_history.remove(0);
        }

        // Clear redo history when new changes are made
        self.redo_history.clear();
    }

    /// Undo the last change.
    pub fn undo(&mut self, current_content: String) -> Option<String> {
        if let Some(prev) = self.undo_history.pop() {
            self.redo_history.push(current_content);
            Some(prev)
        } else {
            None
        }
    }

    /// Redo the last undone change.
    pub fn redo(&mut self, current_content: String) -> Option<String> {
        if let Some(next) = self.redo_history.pop() {
            self.undo_history.push(current_content);
            Some(next)
        } else {
            None
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_history.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_history.is_empty()
    }
}

impl Default for MarkdownEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = MarkdownEditor::new();
        assert_eq!(editor.cursor_position(), 0);
        assert_eq!(editor.selection(), None);
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn test_cursor_management() {
        let mut editor = MarkdownEditor::new();
        editor.set_cursor_position(10);
        assert_eq!(editor.cursor_position(), 10);
    }

    #[test]
    fn test_selection_management() {
        let mut editor = MarkdownEditor::new();
        editor.set_selection(5, 15);
        assert_eq!(editor.selection(), Some((5, 15)));

        editor.clear_selection();
        assert_eq!(editor.selection(), None);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = MarkdownEditor::new();

        // Simulate change from "first state" to "second state"
        editor.add_to_history("first state".to_string());

        assert!(editor.can_undo());
        assert!(!editor.can_redo());

        // Undo from "second state"
        let undone = editor.undo("second state".to_string()).unwrap();
        assert_eq!(undone, "first state");
        assert!(editor.can_redo());

        // Redo from "first state"
        let redone = editor.redo("first state".to_string()).unwrap();
        assert_eq!(redone, "second state");
        assert!(!editor.can_redo());
    }
}
