//! # Writing statistics module for the Markdown Editor plugin
//!
//! This module provides comprehensive writing statistics and analysis for the
//! Markdown editor plugin. It tracks various metrics such as word count,
//! character count, reading time, and writing patterns to help authors
//! monitor their progress and improve their writing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Comprehensive writing statistics for a document.
///
/// The [`WritingStats`] struct provides detailed metrics about a text document
/// including counts, timing information, and readability analysis.
///
/// # Example
///
/// ```rust
/// use cosmarium_markdown_editor::stats::WritingStats;
///
/// let mut stats = WritingStats::new();
/// stats.update("# Hello World\n\nThis is a test document with some content.");
/// assert!(stats.word_count() > 0);
/// assert!(stats.char_count() > 0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingStats {
    /// Total word count
    word_count: usize,
    /// Total character count (including spaces)
    char_count: usize,
    /// Character count (excluding spaces)
    char_count_no_spaces: usize,
    /// Total paragraph count
    paragraph_count: usize,
    /// Total sentence count
    sentence_count: usize,
    /// Average words per sentence
    avg_words_per_sentence: f32,
    /// Average characters per word
    avg_chars_per_word: f32,
    /// Estimated reading time in minutes
    reading_time_minutes: f32,
    /// Most frequent words
    word_frequency: HashMap<String, usize>,
    /// Last update time
    last_updated: SystemTime,
    /// Session statistics
    session_stats: SessionStats,
}

/// Session-based writing statistics.
///
/// Tracks statistics for the current writing session, including
/// time spent writing and words written in this session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// When the session started
    session_start: SystemTime,
    /// Words written in this session
    session_words: usize,
    /// Time spent actively writing (excluding pauses)
    active_writing_time: Duration,
    /// Last activity timestamp
    last_activity: SystemTime,
    /// Words per minute in this session
    words_per_minute: f32,
}

impl WritingStats {
    /// Create a new WritingStats instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let stats = WritingStats::new();
    /// assert_eq!(stats.word_count(), 0);
    /// ```
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            word_count: 0,
            char_count: 0,
            char_count_no_spaces: 0,
            paragraph_count: 0,
            sentence_count: 0,
            avg_words_per_sentence: 0.0,
            avg_chars_per_word: 0.0,
            reading_time_minutes: 0.0,
            word_frequency: HashMap::new(),
            last_updated: now,
            session_stats: SessionStats::new(now),
        }
    }

    /// Update statistics based on the provided text content.
    ///
    /// This method analyzes the text and updates all statistical metrics.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content to analyze
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Hello world! This is a test.");
    /// assert_eq!(stats.word_count(), 6);
    /// assert_eq!(stats.sentence_count(), 2);
    /// ```
    pub fn update(&mut self, content: &str) {
        let old_word_count = self.word_count;
        
        // Update basic counts
        self.word_count = Self::count_words(content);
        self.char_count = content.chars().count();
        self.char_count_no_spaces = content.chars().filter(|&c| c != ' ').count();
        self.paragraph_count = Self::count_paragraphs(content);
        self.sentence_count = Self::count_sentences(content);
        
        // Calculate averages
        if self.sentence_count > 0 {
            self.avg_words_per_sentence = self.word_count as f32 / self.sentence_count as f32;
        }
        
        if self.word_count > 0 {
            self.avg_chars_per_word = self.char_count_no_spaces as f32 / self.word_count as f32;
        }
        
        // Calculate reading time (assuming 200 words per minute)
        self.reading_time_minutes = self.word_count as f32 / 200.0;
        
        // Update word frequency
        self.update_word_frequency(content);
        
        // Update session statistics
        let words_added = self.word_count.saturating_sub(old_word_count);
        self.session_stats.add_words(words_added);
        
        self.last_updated = SystemTime::now();
    }

    /// Get the total word count.
    ///
    /// # Returns
    ///
    /// Number of words in the document.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Hello world");
    /// assert_eq!(stats.word_count(), 2);
    /// ```
    pub fn word_count(&self) -> usize {
        self.word_count
    }

    /// Get the total character count (including spaces).
    ///
    /// # Returns
    ///
    /// Number of characters in the document.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Hi!");
    /// assert_eq!(stats.char_count(), 3);
    /// ```
    pub fn char_count(&self) -> usize {
        self.char_count
    }

    /// Get the character count excluding spaces.
    ///
    /// # Returns
    ///
    /// Number of non-space characters in the document.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Hi there!");
    /// assert_eq!(stats.char_count_no_spaces(), 8); // "Hithere!"
    /// ```
    pub fn char_count_no_spaces(&self) -> usize {
        self.char_count_no_spaces
    }

    /// Get the total paragraph count.
    ///
    /// # Returns
    ///
    /// Number of paragraphs in the document.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("First paragraph.\n\nSecond paragraph.");
    /// assert_eq!(stats.paragraph_count(), 2);
    /// ```
    pub fn paragraph_count(&self) -> usize {
        self.paragraph_count
    }

    /// Get the total sentence count.
    ///
    /// # Returns
    ///
    /// Number of sentences in the document.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("First sentence. Second sentence!");
    /// assert_eq!(stats.sentence_count(), 2);
    /// ```
    pub fn sentence_count(&self) -> usize {
        self.sentence_count
    }

    /// Get the average words per sentence.
    ///
    /// # Returns
    ///
    /// Average number of words per sentence.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Short sentence. This is a longer sentence.");
    /// assert!(stats.avg_words_per_sentence() > 0.0);
    /// ```
    pub fn avg_words_per_sentence(&self) -> f32 {
        self.avg_words_per_sentence
    }

    /// Get the average characters per word.
    ///
    /// # Returns
    ///
    /// Average number of characters per word.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Hi there");
    /// assert!(stats.avg_chars_per_word() > 0.0);
    /// ```
    pub fn avg_chars_per_word(&self) -> f32 {
        self.avg_chars_per_word
    }

    /// Get the estimated reading time in minutes.
    ///
    /// Based on an average reading speed of 200 words per minute.
    ///
    /// # Returns
    ///
    /// Estimated reading time in minutes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("word ".repeat(100).trim()); // 100 words
    /// assert!(stats.reading_time_minutes() > 0.0);
    /// ```
    pub fn reading_time_minutes(&self) -> f32 {
        self.reading_time_minutes
    }

    /// Get the word frequency map.
    ///
    /// # Returns
    ///
    /// Reference to the word frequency HashMap.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("the cat sat on the mat");
    /// let freq = stats.word_frequency();
    /// assert_eq!(freq.get("the"), Some(&2));
    /// ```
    pub fn word_frequency(&self) -> &HashMap<String, usize> {
        &self.word_frequency
    }

    /// Get the most frequent words.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of words to return
    ///
    /// # Returns
    ///
    /// Vector of (word, count) tuples sorted by frequency (descending).
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("the cat sat on the mat the");
    /// let top_words = stats.most_frequent_words(3);
    /// assert_eq!(top_words[0].0, "the");
    /// assert_eq!(top_words[0].1, 3);
    /// ```
    pub fn most_frequent_words(&self, limit: usize) -> Vec<(String, usize)> {
        let mut word_vec: Vec<_> = self.word_frequency.iter()
            .map(|(word, count)| (word.clone(), *count))
            .collect();
        
        word_vec.sort_by(|a, b| b.1.cmp(&a.1));
        word_vec.truncate(limit);
        word_vec
    }

    /// Get session statistics.
    ///
    /// # Returns
    ///
    /// Reference to the session statistics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Some content");
    /// let session = stats.session_stats();
    /// assert!(session.session_words() > 0);
    /// ```
    pub fn session_stats(&self) -> &SessionStats {
        &self.session_stats
    }

    /// Reset session statistics.
    ///
    /// This starts a new writing session while preserving document statistics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cosmarium_markdown_editor::stats::WritingStats;
    ///
    /// let mut stats = WritingStats::new();
    /// stats.update("Some content");
    /// stats.reset_session();
    /// assert_eq!(stats.session_stats().session_words(), 0);
    /// ```
    pub fn reset_session(&mut self) {
        self.session_stats = SessionStats::new(SystemTime::now());
    }

    /// Count words in the given text.
    ///
    /// This method handles Markdown syntax and provides accurate word counts
    /// by excluding markup elements.
    fn count_words(text: &str) -> usize {
        text.split_whitespace()
            .filter(|word| !word.is_empty())
            .map(|word| {
                // Remove common Markdown syntax
                let cleaned = word
                    .trim_matches(|c: char| ".,!?;:()[]{}\"'`*_~".contains(c))
                    .trim_start_matches('#')
                    .trim_start_matches('-')
                    .trim_start_matches('+')
                    .trim_start_matches('>')
                    .trim();
                
                // Skip if it's just markup
                if cleaned.is_empty() || cleaned.chars().all(|c| "*_~`#-+=|".contains(c)) {
                    0
                } else {
                    1
                }
            })
            .sum()
    }

    /// Count paragraphs in the given text.
    fn count_paragraphs(text: &str) -> usize {
        if text.trim().is_empty() {
            return 0;
        }
        
        text.split("\n\n")
            .filter(|paragraph| !paragraph.trim().is_empty())
            .count()
            .max(1) // At least one paragraph if there's content
    }

    /// Count sentences in the given text.
    fn count_sentences(text: &str) -> usize {
        let sentence_endings = ['.', '!', '?'];
        let mut count = 0;
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if sentence_endings.contains(&ch) {
                // Check if it's not an abbreviation or decimal
                if let Some(&next_ch) = chars.peek() {
                    if next_ch.is_whitespace() || next_ch == '\n' {
                        count += 1;
                    }
                } else {
                    count += 1; // End of text
                }
            }
        }
        
        count.max(if text.trim().is_empty() { 0 } else { 1 })
    }

    /// Update word frequency based on the text content.
    fn update_word_frequency(&mut self, text: &str) {
        self.word_frequency.clear();
        
        for word in text.split_whitespace() {
            let lowercase_word = word.to_lowercase();
            let cleaned = lowercase_word
                .trim_matches(|c: char| ".,!?;:()[]{}\"'`*_~".contains(c))
                .trim_start_matches('#')
                .trim();
            
            if !cleaned.is_empty() && cleaned.len() > 2 {
                *self.word_frequency.entry(cleaned.to_string()).or_insert(0) += 1;
            }
        }
    }
}

impl Default for WritingStats {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStats {
    /// Create a new SessionStats instance.
    pub fn new(start_time: SystemTime) -> Self {
        Self {
            session_start: start_time,
            session_words: 0,
            active_writing_time: Duration::new(0, 0),
            last_activity: start_time,
            words_per_minute: 0.0,
        }
    }

    /// Add words to the session count.
    pub fn add_words(&mut self, word_count: usize) {
        if word_count > 0 {
            self.session_words += word_count;
            let now = SystemTime::now();
            
            // Update active writing time if activity is recent
            if let Ok(since_last) = now.duration_since(self.last_activity) {
                if since_last < Duration::from_secs(30) { // 30 second pause threshold
                    self.active_writing_time += since_last;
                }
            }
            
            self.last_activity = now;
            self.update_words_per_minute();
        }
    }

    /// Get the number of words written in this session.
    pub fn session_words(&self) -> usize {
        self.session_words
    }

    /// Get the active writing time for this session.
    pub fn active_writing_time(&self) -> Duration {
        self.active_writing_time
    }

    /// Get the words per minute for this session.
    pub fn words_per_minute(&self) -> f32 {
        self.words_per_minute
    }

    /// Get the session duration (total time since start).
    pub fn session_duration(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.session_start)
            .unwrap_or_default()
    }

    /// Update the words per minute calculation.
    fn update_words_per_minute(&mut self) {
        let minutes = self.active_writing_time.as_secs_f32() / 60.0;
        if minutes > 0.0 {
            self.words_per_minute = self.session_words as f32 / minutes;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writing_stats_creation() {
        let stats = WritingStats::new();
        assert_eq!(stats.word_count(), 0);
        assert_eq!(stats.char_count(), 0);
        assert_eq!(stats.paragraph_count(), 0);
        assert_eq!(stats.sentence_count(), 0);
    }

    #[test]
    fn test_basic_counting() {
        let mut stats = WritingStats::new();
        stats.update("Hello world! This is a test.");
        
        assert_eq!(stats.word_count(), 6);
        assert_eq!(stats.char_count(), 28);
        assert_eq!(stats.sentence_count(), 2);
        assert_eq!(stats.paragraph_count(), 1);
    }

    #[test]
    fn test_markdown_word_counting() {
        let mut stats = WritingStats::new();
        stats.update("# Title\n\n**Bold text** and *italic text* here.");
        
        // Should count words excluding markdown syntax
        assert_eq!(stats.word_count(), 7); // Title, Bold, text, and, italic, text, here
    }

    #[test]
    fn test_paragraph_counting() {
        let mut stats = WritingStats::new();
        stats.update("First paragraph.\n\nSecond paragraph.\n\nThird paragraph.");
        
        assert_eq!(stats.paragraph_count(), 3);
    }

    #[test]
    fn test_sentence_counting() {
        let mut stats = WritingStats::new();
        stats.update("First sentence. Second sentence! Third sentence?");
        
        assert_eq!(stats.sentence_count(), 3);
    }

    #[test]
    fn test_word_frequency() {
        let mut stats = WritingStats::new();
        stats.update("the cat sat on the mat the");
        
        let frequency = stats.word_frequency();
        assert_eq!(frequency.get("the"), Some(&3));
        assert_eq!(frequency.get("cat"), Some(&1));
        assert_eq!(frequency.get("mat"), Some(&1));
    }

    #[test]
    fn test_most_frequent_words() {
        let mut stats = WritingStats::new();
        stats.update("the cat sat on the mat the dog");
        
        let top_words = stats.most_frequent_words(3);
        assert_eq!(top_words[0], ("the".to_string(), 3));
        assert_eq!(top_words.len(), 3);
    }

    #[test]
    fn test_reading_time_calculation() {
        let mut stats = WritingStats::new();
        let text = "word ".repeat(200); // 200 words
        stats.update(&text);
        
        // Should be approximately 1 minute for 200 words
        assert!((stats.reading_time_minutes() - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_averages() {
        let mut stats = WritingStats::new();
        stats.update("Short sentence. This is a much longer sentence with more words.");
        
        assert!(stats.avg_words_per_sentence() > 0.0);
        assert!(stats.avg_chars_per_word() > 0.0);
    }

    #[test]
    fn test_session_stats() {
        let mut stats = WritingStats::new();
        stats.update("Some initial content");
        
        let session = stats.session_stats();
        assert!(session.session_words() > 0);
        assert!(session.active_writing_time() >= Duration::new(0, 0));
    }

    #[test]
    fn test_session_reset() {
        let mut stats = WritingStats::new();
        stats.update("Some content");
        
        let original_words = stats.word_count();
        let original_session_words = stats.session_stats().session_words();
        
        stats.reset_session();
        
        // Document stats should remain
        assert_eq!(stats.word_count(), original_words);
        // Session stats should reset
        assert_eq!(stats.session_stats().session_words(), 0);
    }

    #[test]
    fn test_empty_text() {
        let mut stats = WritingStats::new();
        stats.update("");
        
        assert_eq!(stats.word_count(), 0);
        assert_eq!(stats.char_count(), 0);
        assert_eq!(stats.paragraph_count(), 0);
        assert_eq!(stats.sentence_count(), 0);
    }

    #[test]
    fn test_whitespace_only() {
        let mut stats = WritingStats::new();
        stats.update("   \n\n   \t  ");
        
        assert_eq!(stats.word_count(), 0);
        assert_eq!(stats.paragraph_count(), 0);
        assert_eq!(stats.sentence_count(), 0);
    }
}