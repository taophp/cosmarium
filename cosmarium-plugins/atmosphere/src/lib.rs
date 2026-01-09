use cosmarium_plugin_api::{Plugin, PluginContext, PluginInfo, PluginType, Result};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "ml-emotions")]
pub mod downloader;
#[cfg(feature = "ml-emotions")]
pub mod classifier;
#[cfg(feature = "ml-emotions")]
pub mod color;

#[cfg(feature = "ml-emotions")]
use classifier::{EmotionClassifier, emotions_to_sentiment, emotions_to_palette, EmotionResult};
#[cfg(feature = "ml-emotions")]
use color::AtmospherePalette;

#[cfg(feature = "ml-emotions")]
use lru::LruCache;
#[cfg(feature = "ml-emotions")]
use std::num::NonZeroUsize;
#[cfg(feature = "ml-emotions")]
use std::hash::{Hash, Hasher};
#[cfg(feature = "ml-emotions")]
use strsim::levenshtein;
use serde::{Serialize, Deserialize};

#[cfg(feature = "ml-emotions")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphAnalysis {
    pub sentiment: f32,
    pub emotions: Vec<EmotionResult>,
    pub palette: Option<AtmospherePalette>,
    pub override_palette: Option<AtmospherePalette>,
}

#[cfg(feature = "ml-emotions")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphAnalysisPersistence {
    pub hash: String,
    pub sentiment: String,
    pub emotions_json: String,
    pub palette_json: String,
    pub override_palette_json: Option<String>,
}

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
    pending_sentiment: Arc<Mutex<Option<(f32, Vec<EmotionResult>)>>>,
    #[cfg(feature = "ml-emotions")]
    /// Last detected emotions
    last_emotions: Vec<EmotionResult>,
    #[cfg(feature = "ml-emotions")]
    /// Last detected intensity (max emotion score)
    last_intensity: f32,
    #[cfg(feature = "ml-emotions")]
    /// Current color palette
    current_palette: Option<AtmospherePalette>,
    
    // -- Optimization fields --
    #[cfg(feature = "ml-emotions")]
    /// Cache of paragraph sentiments keyed by content hash
    paragraph_cache: LruCache<u64, ParagraphAnalysis>,
    #[cfg(feature = "ml-emotions")]
    /// Content of the current paragraph during last analysis (for diffing)
    last_analyzed_paragraph: String,
    #[cfg(feature = "ml-emotions")]
    /// Hash of the paragraph currently being analyzed in background
    currently_analyzing_hash: Option<u64>,
    #[cfg(feature = "ml-emotions")]
    /// Last project path seen (to detect changes and load/save cache)
    last_project_path: Option<std::path::PathBuf>,
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
            last_intensity: 0.0,
            #[cfg(feature = "ml-emotions")]
            current_palette: None,
            #[cfg(feature = "ml-emotions")]
            paragraph_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
            #[cfg(feature = "ml-emotions")]
            last_analyzed_paragraph: String::new(),
            #[cfg(feature = "ml-emotions")]
            currently_analyzing_hash: None,
            #[cfg(feature = "ml-emotions")]
            last_project_path: None,
        }
    }
}

impl AtmospherePlugin {
    pub fn new() -> Self {
        let plugin = Self::default();
        
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
    /// Check for pending ML results and apply them
    fn check_pending_analysis(&mut self) {
        if let Ok(mut pending) = self.pending_sentiment.try_lock() {
            if let Some((sentiment, emotions)) = pending.take() {
                self.sentiment = sentiment;
                self.last_emotions = emotions.clone();
                self.last_intensity = emotions.iter().map(|e| e.score).fold(0.0_f32, f32::max);
                
                let palette = emotions_to_palette(&emotions);
                self.current_palette = Some(palette.clone());
                
                // Update cache with result using the hash from when we started the analysis
                if let Some(hash) = self.currently_analyzing_hash.take() {
                    self.paragraph_cache.put(hash, ParagraphAnalysis {
                        sentiment,
                        emotions: emotions.clone(),
                        palette: Some(palette.clone()),
                        override_palette: None,
                    });
                    tracing::debug!("Atmosphere result cached for hash: {}", hash);
                }
                
                tracing::debug!("Applied ML sentiment: {}", sentiment);

                // Auto-save cache occasionally (every result application is a good hook)
                self.save_cache();
            }
        }
    }

    #[cfg(feature = "ml-emotions")]
    /// Load paragraph cache from project metadata
    fn load_cache(&mut self, project_path: &std::path::Path) {
        let cache_file = project_path.join("meta/plugins/atmosphere/cache.toon");
        if cache_file.exists() {
            match std::fs::read_to_string(&cache_file) {
                Ok(content) => {
                    match serde_toon2::from_str::<Vec<ParagraphAnalysisPersistence>>(&content) {
                        Ok(items) => {
                            let count = items.len();
                            for entry in items {
                                if let Ok(hash) = u64::from_str_radix(&entry.hash, 16) {
                                    let sentiment = entry.sentiment.parse::<f32>().unwrap_or(0.0);
                                    let emotions = serde_json::from_str(&entry.emotions_json).unwrap_or_default();
                                    let palette = serde_json::from_str(&entry.palette_json).ok();
                                    let override_palette = entry.override_palette_json.as_ref()
                                        .and_then(|json| serde_json::from_str(json).ok());
                                    
                                    self.paragraph_cache.put(hash, ParagraphAnalysis {
                                        sentiment,
                                        emotions,
                                        palette,
                                        override_palette,
                                    });
                                }
                            }
                            tracing::info!("✓ Loaded {} entries from persistent atmosphere cache", count);
                        }
                        Err(e) => tracing::warn!("Failed to deserialize atmosphere cache: {}", e),
                    }
                }
                Err(e) => tracing::warn!("Failed to read atmosphere cache file: {}", e),
            }
        }
    }

    #[cfg(feature = "ml-emotions")]
    /// Save paragraph cache to project metadata
    fn save_cache(&self) {
        if let Some(ref project_path) = self.last_project_path {
            let plugin_dir = project_path.join("meta/plugins/atmosphere");
            tracing::info!("Atmosphere: Saving cache to {:?}", plugin_dir);
            if let Err(e) = std::fs::create_dir_all(&plugin_dir) {
                tracing::error!("Failed to create atmosphere plugin directory: {}", e);
                return;
            }

            let cache_file = plugin_dir.join("cache.toon");
            // Convert LRU cache to a serializable vector of safe persistent entries
            let mut items = Vec::new();
            for (hash, data) in self.paragraph_cache.iter() {
                items.push(ParagraphAnalysisPersistence {
                    hash: format!("{:x}", hash),
                    sentiment: format!("{:.4}", data.sentiment),
                    emotions_json: serde_json::to_string(&data.emotions).unwrap_or_else(|_| "[]".to_string()),
                    palette_json: serde_json::to_string(&data.palette).unwrap_or_else(|_| "null".to_string()),
                    override_palette_json: data.override_palette.as_ref()
                        .and_then(|p| serde_json::to_string(p).ok()),
                });
            }

            match serde_toon2::to_string(&items) {
                Ok(content) => {
                    if let Err(e) = std::fs::write(&cache_file, content) {
                        tracing::error!("Failed to write atmosphere cache: {}", e);
                    } else {
                        tracing::debug!("Atmosphere cache saved ({} entries)", items.len());
                    }
                }
                Err(e) => tracing::error!("Failed to serialize atmosphere cache: {}", e),
            }
        }
    }

    #[cfg(feature = "ml-emotions")]
    /// Analyze sentiment using ML in a separate thread (non-blocking)
    fn analyze_sentiment_ml_async(&mut self, content: String, hash: u64, relative_cursor: usize, p_idx: usize) {
        // Check if analysis is already running
        if self.analysis_in_progress.load(Ordering::Relaxed) {
            return; // Skip this analysis, previous one still running
        }
        
        // Track what we are analyzing to cache it correctly later
        self.currently_analyzing_hash = Some(hash);
        
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
            tracing::info!("[P#{}] ML Emotion analysis started...", p_idx);
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
                        let palette = emotions_to_palette(&emotions);
                        
                        tracing::info!(
                            "✓ [P#{}] ML Emotion analysis complete: sentiment={:.2}, dominant={} ({} @ H:{:.0} S:{:.1} L:{:.1})", 
                            p_idx, sentiment, emotions[0].emotion, palette.color_name, palette.main_bg_h, palette.main_bg_s, palette.main_bg_l
                        );

                        if let Ok(mut pending) = result_arc.lock() {
                            *pending = Some((sentiment, emotions));
                        }
                    } else {
                        tracing::info!("✓ [P#{}] ML Emotion analysis complete: neutral", p_idx);
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
    // Helper to get paragraph bounds and content using byte indices
    // Dialogue discovery: If the paragraph starts with a dialogue marker, 
    // it will try to swallow contiguous dialogue paragraphs for a stable "scene" context.
    fn get_current_paragraph(content: &str, cursor_byte_idx: usize) -> (usize, usize, &str) {
        let cursor_byte_idx = cursor_byte_idx.min(content.len());
        
        // Helper to check for dialogue marker at start
        let is_dialogue = |p_text: &str| -> bool {
            let t = p_text.trim_start();
            t.starts_with('-') || t.starts_with('—') || t.starts_with('–')
        };
        
        let mut start = content[..cursor_byte_idx].rfind("\n\n").map(|i| i + 2).unwrap_or(0);
        let mut end = content[cursor_byte_idx..].find("\n\n").map(|i| cursor_byte_idx + i).unwrap_or(content.len());
        
        let current_p = &content[start..end];
        
        // If current paragraph is dialogue, expand context to include surrounding dialogue paragraphs
        if is_dialogue(current_p) {
            // Expand upwards (max 2)
            for _ in 0..2 {
                if start <= 2 { break; }
                let check_before = &content[..start - 2];
                let prev_start = check_before.rfind("\n\n").map(|i| i + 2).unwrap_or(0);
                let prev_p = &content[prev_start..start - 2];
                if is_dialogue(prev_p) {
                    start = prev_start;
                } else {
                    break;
                }
            }
            
            // Expand downwards (max 2)
            for _ in 0..2 {
                if end + 2 >= content.len() { break; }
                let check_after = &content[end + 2..];
                let next_end_rel = check_after.find("\n\n").unwrap_or(check_after.len());
                let next_end = end + 2 + next_end_rel;
                let next_p = &content[end + 2..next_end];
                if is_dialogue(next_p) {
                    end = next_end;
                } else {
                    break;
                }
            }
        }
        
        // Trim boundaries for stability
        let slice = &content[start..end];
        let trimmed = slice.trim();
        
        if trimmed.is_empty() {
            return (start, end, "");
        }
        
        // Adjust start/end to match trimmed content
        let lead_space = slice.len() - slice.trim_start().len();
        let trail_space = slice.len() - slice.trim_end().len();
        
        (start + lead_space, end - trail_space, trimmed)
    }
    
    #[cfg(feature = "ml-emotions")]
    fn get_previous_paragraph_sentiment(&mut self, content: &str, p_start: usize) -> Option<(f32, f32, Option<AtmospherePalette>)> {
        let text_before = &content[..p_start];
        let prev_p_end = text_before.trim_end().rfind('\n').map(|i| i + 1).unwrap_or(0);
        let (_, _, prev_p_content) = Self::get_current_paragraph(content, prev_p_end.saturating_sub(1));
        
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        prev_p_content.hash(&mut hasher);
        let hash = hasher.finish();

        self.paragraph_cache.get(&hash).map(|analysis| {
            let intensity = analysis.emotions.iter().map(|er| er.score).fold(0.0_f32, f32::max);
            (analysis.sentiment, intensity, analysis.palette.clone())
        })
    }

    #[cfg(feature = "ml-emotions")]
    fn get_next_paragraph_sentiment(&mut self, content: &str, current_end: usize) -> Option<(f32, f32, Option<AtmospherePalette>)> {
        if current_end >= content.len() { return None; }
        
        let (_, _, next_p_content) = Self::get_current_paragraph(content, current_end + 1);
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        next_p_content.hash(&mut hasher);
        let hash = hasher.finish();

        self.paragraph_cache.get(&hash).map(|analysis| {
            let intensity = analysis.emotions.iter().map(|er| er.score).fold(0.0_f32, f32::max);
            (analysis.sentiment, intensity, analysis.palette.clone())
        })
    }

    fn get_paragraph_index(content: &str, byte_idx: usize) -> usize {
        let text_before = &content[..byte_idx.min(content.len())];
        // Count \n\n occurrences
        text_before.matches("\n\n").count() + 1
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

#[async_trait]
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
        #[cfg(feature = "ml-emotions")]
        {
            // 0. Update project path tracking and load cache if needed
            if let Some(current_path) = ctx.project_path() {
                if self.last_project_path.as_ref() != Some(&current_path) {
                    tracing::info!("Atmosphere: Project changed to {:?}, loading cache...", current_path);
                    self.load_cache(&current_path);
                    self.last_project_path = Some(current_path);
                }
            } else if self.last_project_path.is_some() {
                tracing::debug!("Atmosphere: Project closed, clearing path tracker");
                self.last_project_path = None;
            }

            self.check_pending_analysis();
        }

        if let Some(content) = ctx.get_shared_state::<String>("markdown_editor_content") {
            let cursor_idx = ctx
                .get_shared_state::<usize>("markdown_editor_cursor_idx")
                .unwrap_or(0);
            
            // Only proceed if cursor or content changed significantly?
            // Actually, we check per frame but logic inside filters it.
            
            #[cfg(feature = "ml-emotions")]
            {
                // Safely convert character index to byte index
                let cursor_byte_idx = content
                    .char_indices()
                    .nth(cursor_idx)
                    .map(|(i, _)| i)
                    .unwrap_or(content.len());

                let (p_start, p_end, p_content) = Self::get_current_paragraph(&content, cursor_byte_idx);
                
                // 1. Navigation Caching
                use std::collections::hash_map::DefaultHasher;
                let mut hasher = DefaultHasher::new();
                p_content.hash(&mut hasher);
                let p_hash = hasher.finish();
                
                // Publish current paragraph hash for UI coordination (e.g. manual override)
                ctx.set_shared_state("atmosphere_paragraph_hash", p_hash);

                // 0. Handle manual override requests from UI
                if let Some(manual_palette) = ctx.get_shared_state::<AtmospherePalette>("atmosphere_manual_palette_request") {
                    tracing::info!("Atmosphere: Manual palette override requested for paragraph {}", p_hash);
                    
                    // Update cache with override
                    let mut analysis = self.paragraph_cache.get(&p_hash).cloned().unwrap_or_else(|| ParagraphAnalysis {
                        sentiment: 0.0,
                        emotions: Vec::new(),
                        palette: None,
                        override_palette: None,
                    });
                    
                    analysis.override_palette = Some(manual_palette.clone());
                    self.paragraph_cache.put(p_hash, analysis);
                    
                    // Apply immediately
                    self.current_palette = Some(manual_palette);
                    
                    // Clear the request
                    ctx.set_shared_state::<Option<AtmospherePalette>>("atmosphere_manual_palette_request", None);
                    
                    // Save cache
                    self.save_cache();
                }

                // Handle clear manual override request
                if ctx.get_shared_state::<bool>("atmosphere_clear_manual_request").unwrap_or(false) {
                    tracing::info!("Atmosphere: Clearing manual override for paragraph {}", p_hash);
                    if let Some(analysis) = self.paragraph_cache.get_mut(&p_hash) {
                        analysis.override_palette = None;
                        self.current_palette = analysis.palette.clone();
                    }
                    ctx.set_shared_state("atmosphere_clear_manual_request", false);
                    self.save_cache();
                }

                if let Some(analysis) = self.paragraph_cache.get(&p_hash) {
                    tracing::debug!("Atmosphere Cache HIT for paragraph hash: {}", p_hash);
                    self.sentiment = analysis.sentiment;
                    self.last_emotions = analysis.emotions.clone();
                    self.last_intensity = analysis.emotions.iter().map(|e| e.score).fold(0.0_f32, f32::max);
                    self.current_palette = analysis.override_palette.clone().or_else(|| analysis.palette.clone());
                    
                    // Update tracked content to match the cache hit (ensures stability when starting to edit)
                    self.last_analyzed_paragraph = p_content.to_string();
                } else if p_content.len() < 50 {
                    // 2. Bidirectional Inheritance (Short new paragraphs)
                    let prev = self.get_previous_paragraph_sentiment(&content, p_start);
                    let next = self.get_next_paragraph_sentiment(&content, p_end);
                    
                    match (prev, next) {
                        (Some((ps, pi, pp)), Some((ns, ni, np))) => {
                            tracing::debug!("Averaging sentiment from prev/next paragraphs (length {})", p_content.len());
                            self.sentiment = (ps + ns) / 2.0;
                            self.last_intensity = (pi + ni) / 2.0;
                            // For palette, take the previous one as it's more likely to be the "scene start"
                            self.current_palette = pp.or(np);
                        }
                        (Some((s, i, p)), None) | (None, Some((s, i, p))) => {
                            tracing::debug!("Inheriting sentiment from neighbor paragraph (length {})", p_content.len());
                            self.sentiment = s;
                            self.last_intensity = i;
                            self.current_palette = p;
                        }
                        (None, None) => {
                            // Keep current or defaults
                        }
                    }
                    self.last_analyzed_paragraph = String::new();
                } else {
                    // 3. Editing Threshold (Only for paragraphs >= 50 chars)
                    // If the hash changed (cache miss), check if it's worth re-analyzing
                    let dist = levenshtein(&self.last_analyzed_paragraph, p_content);
                    let threshold = (self.last_analyzed_paragraph.len().max(p_content.len()) as f32 * 0.05).max(5.0) as usize;
                    
                    if dist > threshold || self.last_analyzed_paragraph.is_empty() {
                         tracing::debug!("Change threshold exceeded (dist: {}/threshold: {}), triggering analysis", dist, threshold);
                          let relative_cursor = cursor_byte_idx.saturating_sub(p_start);
                          let p_idx = Self::get_paragraph_index(&content, cursor_byte_idx);
                          ctx.set_shared_state("atmosphere_paragraph_idx", p_idx);
                          
                          self.analyze_sentiment_ml_async(p_content.to_string(), p_hash, relative_cursor, p_idx);
                          // Update local tracker immediately to prevent spamming
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
            ctx.set_shared_state("atmosphere_intensity", self.last_intensity);
            ctx.set_shared_state("atmosphere_analyzing", self.analysis_in_progress.load(Ordering::Relaxed));
            ctx.set_shared_state("atmosphere_emotions", self.last_emotions.clone());
            
            if let Some(palette) = &self.current_palette {
                if let Ok(palette_json) = serde_json::to_string(palette) {
                    ctx.set_shared_state("atmosphere_palette", palette_json);
                    ctx.set_shared_state("atmosphere_current_emotion", palette.color_name.clone());
                }
            }
            
            // Map EmotionResult back to (String, f32) for legacy shared state if needed,
            // or just publish the EmotionResult list if serializable.
            // For now, let's keep atmosphere_emotions as Vec<(String, f32)> for compatibility
            let compat_emotions: Vec<(String, f32)> = self.last_emotions.iter()
                .map(|er| (er.emotion.clone(), er.score))
                .collect();
            ctx.set_shared_state("atmosphere_emotions", compat_emotions);
        }

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let _ = ctx;
        #[cfg(feature = "ml-emotions")]
        self.save_cache();
        Ok(())
    }
}
