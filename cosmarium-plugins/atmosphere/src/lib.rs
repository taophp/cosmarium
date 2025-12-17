use cosmarium_plugin_api::{Plugin, PluginContext, PluginInfo, PluginType, Result};

pub struct AtmospherePlugin {
    /// Current sentiment score (-1.0 to 1.0)
    sentiment: f32,
    /// Last analyzed content hash
    last_content_hash: u64,
    last_cursor_idx: usize,
}

impl Default for AtmospherePlugin {
    fn default() -> Self {
        Self {
            sentiment: 0.0,
            last_content_hash: 0,
            last_cursor_idx: 0,
        }
    }
}

impl AtmospherePlugin {
    pub fn new() -> Self {
        Self::default()
    }

    fn analyze_sentiment_weighted(&mut self, content: &str, relative_cursor: usize) {
        let mut score = 0.0f32;

        // Track byte offset to calculate distance
        let mut current_byte_offset = 0;

        // We need to iterate words and keep track of their position
        // split_whitespace() doesn't give offsets easily.
        // Let's use match_indices or just manual scanning?
        // Simpler: split by whitespace but reconstruct offset? No.
        // Let's just use a simple tokenizer that tracks indices.

        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let total_chars = char_indices.len();

        if total_chars == 0 {
            self.sentiment = 0.0;
            return;
        }

        let mut i = 0;
        while i < total_chars {
            // Skip non-alphanumeric
            while i < total_chars && !char_indices[i].1.is_alphanumeric() {
                i += 1;
            }

            if i >= total_chars {
                break;
            }

            let start_idx = i;
            let start_byte = char_indices[start_idx].0;

            // Consume word
            while i < total_chars && char_indices[i].1.is_alphanumeric() {
                i += 1;
            }
            let end_idx = i;

            // Extract word
            let word_len = end_idx - start_idx;
            // Reconstruct word from slice (safe because we tracked char indices)
            // Actually, we can just use the byte offsets from char_indices
            let end_byte = if end_idx < total_chars {
                char_indices[end_idx].0
            } else {
                content.len()
            };
            let word = &content[start_byte..end_byte];

            let w = word.to_lowercase();

            // Calculate distance from cursor
            // Cursor is at `relative_cursor` (byte offset)
            // Word center is roughly (start_byte + end_byte) / 2
            let word_center = (start_byte + end_byte) / 2;
            let distance = (word_center as isize - relative_cursor as isize).abs() as f32;

            // Weight: 1.0 at cursor, decaying to 0.0 at 500 chars away
            let max_dist = 500.0;
            let weight = (1.0 - (distance / max_dist)).max(0.0);

            // Boost weight for very close words (immediate context)
            let weight = if distance < 50.0 {
                weight * 2.0
            } else {
                weight
            };

            match w.as_str() {
                // Positive / Warm / Light
                "joy" | "happy" | "sun" | "light" | "laugh" | "smile" | "love" | "hope"
                | "bright" | "warm" | "day" | "morning" | "gold" | "white" | "joie" | "heureux"
                | "soleil" | "lumière" | "rire" | "sourire" | "amour" | "espoir" | "brillant"
                | "chaud" | "jour" | "matin" | "or" | "blanc" | "belle" | "beau" => {
                    score += 1.0 * weight
                }
                // Negative / Cold / Dark
                "death" | "sad" | "dark" | "night" | "fear" | "pain" | "cold" | "blood"
                | "shadow" | "cry" | "tear" | "black" | "grey" | "kill" | "die" | "mort"
                | "triste" | "sombre" | "nuit" | "peur" | "douleur" | "froid" | "sang"
                | "ombre" | "pleurer" | "larme" | "noir" | "gris" | "tuer" | "mourir" => {
                    score -= 1.0 * weight
                }
                _ => {}
            }
        }

        // Normalize
        // Multiplier 0.8 ensures a single strong word triggers ~80% of the effect.
        self.sentiment = (score * 0.8f32).clamp(-1.0f32, 1.0f32);

        tracing::debug!(
            "Atmosphere analysis: score={}, sentiment={}",
            score,
            self.sentiment
        );
    }

    fn analyze_sentiment(&mut self, content: &str) {
        self.analyze_sentiment_weighted(content, content.len() / 2);
    }

    fn update_theme(&self, _ctx: &mut PluginContext) {
        // Theme updates are handled by the main app consuming the shared state
    }
}

impl Plugin for AtmospherePlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo::new(
            "atmosphere",
            "0.1.0",
            "Dynamic Atmosphere",
            "Adjusts UI theme based on content sentiment",
        )
    }

    fn initialize(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        tracing::info!("Atmosphere plugin initialized");
        Ok(())
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Theme
    }

    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Check for content updates
        if let Some(content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            let cursor_idx = ctx
                .get_shared_state::<usize>("markdown_editor_cursor_idx")
                .unwrap_or(0);

            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            let new_hash = hasher.finish();

            // Update if content changed OR cursor moved significantly
            // We don't want to re-analyze on every single character move if it's expensive,
            // but for this POC it's fine.
            if new_hash != self.last_content_hash || cursor_idx != self.last_cursor_idx {
                // Extract context window around cursor
                let window_size = 1000;
                let start = cursor_idx.saturating_sub(window_size / 2);
                let end = (cursor_idx + window_size / 2).min(content.len());

                // Ensure valid UTF-8 boundaries
                let start = if let Some((i, _)) = content.char_indices().find(|(i, _)| *i >= start)
                {
                    i
                } else {
                    start
                };
                let end = if let Some((i, _)) = content.char_indices().find(|(i, _)| *i >= end) {
                    i
                } else {
                    end
                };

                // Safety check for slicing
                let slice = if start <= end && end <= content.len() {
                    &content[start..end]
                } else {
                    &content[..] // Fallback to whole content if indices are weird
                };

                tracing::debug!(
                    "AtmospherePlugin: analyzing window [{}..{}] (cursor={})",
                    start,
                    end,
                    cursor_idx
                );
                tracing::debug!("AtmospherePlugin: window content: {:?}", slice);

                // Analyze with distance weighting
                // We pass the cursor relative position within the slice
                let relative_cursor = cursor_idx.saturating_sub(start);
                self.analyze_sentiment_weighted(slice, relative_cursor);

                self.last_content_hash = new_hash;
                self.last_cursor_idx = cursor_idx;

                // Publish sentiment for App to consume
                ctx.set_shared_state("atmosphere_sentiment", self.sentiment);
            }
        }
        Ok(())
    }
}
