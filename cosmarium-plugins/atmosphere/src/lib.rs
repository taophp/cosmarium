use cosmarium_plugin_api::{Plugin, PluginContext, PluginInfo, PluginType, Result};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "ml-emotions")]
pub mod downloader;
#[cfg(feature = "ml-emotions")]
pub mod classifier;

#[cfg(feature = "ml-emotions")]
use classifier::{EmotionClassifier, emotions_to_sentiment};

#[cfg(feature = "ml-emotions")]
use lru::LruCache;
#[cfg(feature = "ml-emotions")]
use std::num::NonZeroUsize;
#[cfg(feature = "ml-emotions")]
use std::hash::{Hash, Hasher};
#[cfg(feature = "ml-emotions")]
use strsim::levenshtein;

pub struct AtmospherePlugin {
    /// Current sentiment score (-1.0 to 1.0)
    sentiment: f32,
    #[cfg(not(feature = "ml-emotions"))]
    /// Last analyzed content hash
    last_content_hash: u64,
    #[cfg(not(feature = "ml-emotions"))]
    last_cursor_idx: usize,
    
    #[cfg(feature = "ml-emotions")]
    /// ML classifier (loaded in background)
    classifier: Arc<Mutex<Option<EmotionClassifier>>>,
    #[cfg(feature = "ml-emotions")]
    /// Flag indicating if analysis is currently running
    analysis_in_progress: Arc<AtomicBool>,
    #[cfg(feature = "ml-emotions")]
    /// Shared sentiment result from worker thread (sentiment, top_emotions)
    pending_sentiment: Arc<Mutex<Option<(f32, Vec<(String, f32)>)>>>,
    #[cfg(feature = "ml-emotions")]
    /// Last detected emotions
    last_emotions: Vec<(String, f32)>,
    #[cfg(feature = "ml-emotions")]
    /// Whether ML is available
    ml_available: bool,
    
    // -- Optimization fields --
    #[cfg(feature = "ml-emotions")]
    /// Cache of paragraph sentiments keyed by content hash
    paragraph_cache: LruCache<u64, (f32, Vec<(String, f32)>)>,
    #[cfg(feature = "ml-emotions")]
    /// Content of the current paragraph during last analysis (for diffing)
    last_analyzed_paragraph: String,
}

impl Default for AtmospherePlugin {
    fn default() -> Self {
        Self {
            sentiment: 0.0,
            #[cfg(not(feature = "ml-emotions"))]
            last_content_hash: 0,
            #[cfg(not(feature = "ml-emotions"))]
            last_cursor_idx: 0,
            #[cfg(feature = "ml-emotions")]
            classifier: Arc::new(Mutex::new(None)),
            #[cfg(feature = "ml-emotions")]
            analysis_in_progress: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "ml-emotions")]
            pending_sentiment: Arc::new(Mutex::new(None)),
            #[cfg(feature = "ml-emotions")]
            last_emotions: Vec::new(),
            #[cfg(feature = "ml-emotions")]
            ml_available: false,
            #[cfg(feature = "ml-emotions")]
            paragraph_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
            #[cfg(feature = "ml-emotions")]
            last_analyzed_paragraph: String::new(),
        }
    }
}

impl AtmospherePlugin {
    pub fn new() -> Self {
        let mut plugin = Self::default();
        
        #[cfg(feature = "ml-emotions")]
        {
            // Start model loading in background
            let classifier_arc = plugin.classifier.clone();
            std::thread::spawn(move || {
                tracing::info!("Loading emotion detection model in background...");
                match downloader::ensure_model_downloaded() {
                    Ok((model_path, tokenizer_path)) => {
                        match EmotionClassifier::new(&model_path, &tokenizer_path) {
                            Ok(classifier) => {
                                tracing::info!("✓ Emotion detection model loaded successfully");
                                let mut lock = classifier_arc.lock().unwrap();
                                *lock = Some(classifier);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to load classifier: {}. Using lexicon fallback.", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to download model: {}. Using lexicon fallback.", e);
                    }
                }
            });
            tracing::info!("Model loading started in background. Using lexicon until ready...");
        }
        
        plugin
    }

    #[cfg(feature = "ml-emotions")]
    /// Analyze sentiment using ML in a separate thread (non-blocking)
    fn analyze_sentiment_ml_async(&mut self, content: String, relative_cursor: usize) {
        // Check if analysis is already running
        if self.analysis_in_progress.load(Ordering::Relaxed) {
            return; // Skip this analysis, previous one still running
        }
        
        // ... (standard pending check)
        if let Ok(mut pending) = self.pending_sentiment.try_lock() {
            if let Some((sentiment, emotions)) = pending.take() {
                self.sentiment = sentiment;
                self.last_emotions = emotions.clone();
                // Update cache with result
                use std::collections::hash_map::DefaultHasher;
                let mut hasher = DefaultHasher::new();
                self.last_analyzed_paragraph.hash(&mut hasher);
                let hash = hasher.finish();
                self.paragraph_cache.put(hash, (sentiment, emotions));
                
                tracing::debug!("Applied ML sentiment: {}", sentiment);
            }
        }
        
        // Try to get classifier
        let classifier_opt = {
            if let Ok(lock) = self.classifier.try_lock() {
                lock.as_ref().map(|_c| true)
            } else {
                None
            }
        };
        
        if classifier_opt.is_none() {
            // Model not ready, use lexicon
            self.analyze_sentiment_lexicon(&content, relative_cursor);
            return;
        }
        
        // Start analysis in background thread
        self.analysis_in_progress.store(true, Ordering::Relaxed);
        
        let classifier_arc = self.classifier.clone();
        let in_progress_flag = self.analysis_in_progress.clone();
        let result_arc = self.pending_sentiment.clone();
        
        std::thread::spawn(move || {
            // ... (worker logic same as before)
            tracing::info!("ML Emotion analysis started...");
            let result = {
                if let Ok(lock) = classifier_arc.lock() {
                    if let Some(ref classifier) = *lock {
                        classifier.classify(&content)
                    } else {
                        Err(anyhow::anyhow!("Classifier not available"))
                    }
                } else {
                    Err(anyhow::anyhow!("Could not lock classifier"))
                }
            };
            
            match result {
                Ok(emotions) => {
                    if !emotions.is_empty() {
                        let sentiment = emotions_to_sentiment(&emotions);
                        
                        // Log detailed results
                        let mut emotion_str = String::new();
                        let mut emotions_vec = Vec::new();
                        for (i, res) in emotions.iter().take(3).enumerate() {
                            if i > 0 { emotion_str.push_str(", "); }
                            emotion_str.push_str(&format!("{}: {:.2}", res.emotion, res.score));
                            emotions_vec.push((res.emotion.clone(), res.score));
                        }
                        tracing::info!("✓ ML Emotion analysis complete: sentiment={:.2}, emotions=[{}]", sentiment, emotion_str);

                        if let Ok(mut pending) = result_arc.lock() {
                            *pending = Some((sentiment, emotions_vec));
                        }
                    } else {
                        tracing::info!("✓ ML Emotion analysis complete: neutral");
                        if let Ok(mut pending) = result_arc.lock() {
                            *pending = Some((0.0, Vec::new()));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("ML analysis failed: {}", e);
                }
            }
            
            // Mark analysis as complete
            in_progress_flag.store(false, Ordering::Relaxed);
        });
    }

    #[cfg(feature = "ml-emotions")]
    // Helper to get paragraph bounds and content
    fn get_current_paragraph(content: &str, cursor_idx: usize) -> (usize, usize, &str) {
        let start = content[..cursor_idx].rfind("\n\n").map(|i| i + 2).unwrap_or(0);
        let end = content[cursor_idx..].find("\n\n").map(|i| cursor_idx + i).unwrap_or(content.len());
        (start, end, &content[start..end])
    }
    
    #[cfg(feature = "ml-emotions")]
    fn get_previous_paragraph_sentiment(&mut self, content: &str, current_start: usize) -> Option<f32> {
        if current_start <= 2 { return None; }
        // Look before the current paragraph (skip the \n\n)
        let slice_before = &content[..current_start-2];
        let prev_start = slice_before.rfind("\n\n").map(|i| i + 2).unwrap_or(0);
        let prev_content = &slice_before[prev_start..];
        
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        prev_content.hash(&mut hasher);
        let hash = hasher.finish();
        
        self.paragraph_cache.get(&hash).map(|(s, _)| *s)
    }

    /// Lexicon-based fallback (original implementation)
    fn analyze_sentiment_lexicon(&mut self, content: &str, relative_cursor: usize) {
        let mut score = 0.0f32;

        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let total_chars = char_indices.len();

        if total_chars == 0 {
            self.sentiment = 0.0;
            return;
        }

        let mut i = 0;
        while i < total_chars {
            while i < total_chars && !char_indices[i].1.is_alphanumeric() {
                i += 1;
            }

            if i >= total_chars {
                break;
            }

            let start_idx = i;
            let start_byte = char_indices[start_idx].0;

            while i < total_chars && char_indices[i].1.is_alphanumeric() {
                i += 1;
            }
            let end_idx = i;

            let end_byte = if end_idx < total_chars {
                char_indices[end_idx].0
            } else {
                content.len()
            };
            let word = &content[start_byte..end_byte];

            let w = word.to_lowercase();

            let word_center = (start_byte + end_byte) / 2;
            let distance = (word_center as isize - relative_cursor as isize).abs() as f32;

            let max_dist = 500.0;
            let weight = (1.0 - (distance / max_dist)).max(0.0);
            let weight = if distance < 50.0 {
                weight * 2.0
            } else {
                weight
            };

            match w.as_str() {
                "joy" | "happy" | "sun" | "light" | "laugh" | "smile" | "love" | "hope"
                | "bright" | "warm" | "day" | "morning" | "gold" | "white" | "joie" | "heureux"
                | "soleil" | "lumière" | "rire" | "sourire" | "amour" | "espoir" | "brillant"
                | "chaud" | "jour" | "matin" | "or" | "blanc" | "belle" | "beau" => {
                    score += 1.0 * weight
                }
                "death" | "sad" | "dark" | "night" | "fear" | "pain" | "cold" | "blood"
                | "shadow" | "cry" | "tear" | "black" | "grey" | "kill" | "die" | "mort"
                | "triste" | "sombre" | "nuit" | "peur" | "douleur" | "froid" | "sang"
                | "ombre" | "pleurer" | "larme" | "noir" | "gris" | "tuer" | "mourir" => {
                    score -= 1.0 * weight
                }
                _ => {}
            }
        }

        self.sentiment = (score * 0.8f32).clamp(-1.0f32, 1.0f32);

        tracing::debug!(
            "Lexicon analysis (fallback): score={}, sentiment={}",
            score,
            self.sentiment
        );
    }
}

impl Plugin for AtmospherePlugin {
    // ... (info, initialize, plugin_type same as before)
    fn info(&self) -> PluginInfo {
        PluginInfo::new(
            "atmosphere",
            "0.1.0",
            "Dynamic Atmosphere",
            "Adjusts UI theme based on content emotions (ML-powered)",
        )
    }

    fn initialize(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        tracing::info!("Atmosphere plugin initialized (ML emotion detection)");
        Ok(())
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Theme
    }

    fn update(&mut self, ctx: &mut PluginContext) -> Result<()> {
        if let Some(content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            let cursor_idx = ctx
                .get_shared_state::<usize>("markdown_editor_cursor_idx")
                .unwrap_or(0);
            
            // Only proceed if cursor or content changed significantly?
            // Actually, we check per frame but logic inside filters it.
            
            #[cfg(feature = "ml-emotions")]
            {
                let (p_start, _, p_content) = Self::get_current_paragraph(&content, cursor_idx);
                
                // 1. Navigation Caching
                use std::collections::hash_map::DefaultHasher;
                let mut hasher = DefaultHasher::new();
                p_content.hash(&mut hasher);
                let p_hash = hasher.finish();
                
                if let Some((cached_sentiment, cached_emotions)) = self.paragraph_cache.get(&p_hash) {
                    tracing::debug!("Atmosphere Cache HIT for paragraph hash: {}", p_hash);
                    self.sentiment = *cached_sentiment;
                    self.last_emotions = cached_emotions.clone();
                    
                    // Update tracked content to match the cache hit
                    self.last_analyzed_paragraph = p_content.to_string();
                } else if p_content.len() < 50 {
                    // 2. Inheritance (Short new paragraphs)
                    // If we don't have a cache hit but paragraph is short, strictly inherit and skip analysis
                    if let Some(prev_sentiment) = self.get_previous_paragraph_sentiment(&content, p_start) {
                        tracing::debug!("Inheriting sentiment from previous paragraph (length {})", p_content.len());
                        self.sentiment = prev_sentiment;
                        // We reset last_analyzed_paragraph to empty so that when it reaches 50, it triggers immediately
                        self.last_analyzed_paragraph = String::new();
                    } else {
                        // If no previous paragraph, keep current sentiment or use neutral
                    }
                } else {
                    // 3. Editing Threshold (Only for paragraphs >= 50 chars)
                    // If the hash changed (cache miss), check if it's worth re-analyzing
                    let dist = levenshtein(&self.last_analyzed_paragraph, p_content);
                    let threshold = (self.last_analyzed_paragraph.len().max(p_content.len()) as f32 * 0.05).max(5.0) as usize;
                    
                    if dist > threshold || self.last_analyzed_paragraph.is_empty() {
                         tracing::debug!("Change threshold exceeded (dist: {}/threshold: {}), triggering analysis", dist, threshold);
                         let relative_cursor = cursor_idx.saturating_sub(p_start);
                         self.analyze_sentiment_ml_async(p_content.to_string(), relative_cursor);
                         // Note: cache update happens when analysis completes in worker
                         self.last_analyzed_paragraph = p_content.to_string();
                    }
                }
            }
             
            #[cfg(not(feature = "ml-emotions"))]
            {
                // Fallback logic (existing code simplified for brevity if needed, or kept)
               // ... existing explicit update check ...
               use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                content.hash(&mut hasher);
                let new_hash = hasher.finish();

                if new_hash != self.last_content_hash || cursor_idx != self.last_cursor_idx {
                    // ... existing window logic ...
                    let window_size = 1000;
                    let start = cursor_idx.saturating_sub(window_size / 2);
                    let end = (cursor_idx + window_size / 2).min(content.len());
                    // ... (snap to word boundaries) ...
                    let slice = &content[start..end]; // simplified
                    let relative_cursor = cursor_idx.saturating_sub(start);
                    self.analyze_sentiment_lexicon(slice, relative_cursor);
                    
                    self.last_content_hash = new_hash;
                    self.last_cursor_idx = cursor_idx;
                }
            }
        }

        // Publish current sentiment and status
        ctx.set_shared_state("atmosphere_sentiment", self.sentiment);
        
        #[cfg(feature = "ml-emotions")]
        {
            let is_analyzing = self.analysis_in_progress.load(Ordering::Relaxed);
            ctx.set_shared_state("atmosphere_analyzing", is_analyzing);
            ctx.set_shared_state("atmosphere_emotions", self.last_emotions.clone());
        }

        Ok(())
    }
}
