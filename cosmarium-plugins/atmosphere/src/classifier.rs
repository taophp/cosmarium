use anyhow::{Context, Result};
use std::path::Path;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

/// Emotion labels from GoEmotions dataset (28 emotions)
const EMOTION_LABELS: [&str; 28] = [
    "admiration", "amusement", "anger", "annoyance", "approval", "caring",
    "confusion", "curiosity", "desire", "disappointment", "disapproval",
    "disgust", "embarrassment", "excitement", "fear", "gratitude", "grief",
    "joy", "love", "nervousness", "optimism", "pride", "realization",
    "relief", "remorse", "sadness", "surprise", "neutral",
];

/// Emotion detection result
#[derive(Debug, Clone)]
pub struct EmotionResult {
    pub emotion: String,
    pub score: f32,
}

/// Tract-based emotion classifier
pub struct EmotionClassifier {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    tokenizer: Tokenizer,
}

impl EmotionClassifier {
    /// Load the model and tokenizer from paths
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        tracing::info!("Loading ONNX model from {:?}", model_path);
        
        let model = tract_onnx::onnx()
            .model_for_path(model_path)
            .context("Failed to load ONNX model")?
            .into_optimized()
            .context("Failed to optimize model")?
            .into_runnable()
            .context("Failed to make model runnable")?;
        
        tracing::info!("Loading tokenizer from {:?}", tokenizer_path);
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        Ok(Self { model, tokenizer })
    }
    
    /// Classify emotions in text and return top 3 results
    pub fn classify(&self, text: &str) -> Result<Vec<EmotionResult>> {
        // Tokenize input
        let encoding = self.tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        
        // Convert to Tract tensors
        let input_ids_array: tract_ndarray::Array2<i64> = tract_ndarray::Array2::from_shape_vec(
            (1, input_ids.len()),
            input_ids.iter().map(|&x| x as i64).collect(),
        )?;
        
        let attention_mask_array: tract_ndarray::Array2<i64> = tract_ndarray::Array2::from_shape_vec(
            (1, attention_mask.len()),
            attention_mask.iter().map(|&x| x as i64).collect(),
        )?;
        
        // Convert to Tensor
        let input_ids_tensor = Tensor::from(input_ids_array);
        let attention_mask_tensor = Tensor::from(attention_mask_array);
        
        // Run inference
        let result = self.model.run(tvec![
            input_ids_tensor.into(),
            attention_mask_tensor.into(),
        ])?;
        
        // Extract logits
        let logits = result[0]
            .to_array_view::<f32>()?;
        
        // Flatten to 1D and apply sigmoid
        let probs: Vec<f32> = logits
            .as_slice()
            .ok_or_else(|| anyhow::anyhow!("Failed to get logits as slice"))?
            .iter()
            .map(|&x| 1.0 / (1.0 + (-x).exp()))
            .collect();
        
        // Get top 3 emotions
        let mut indexed_probs: Vec<(usize, f32)> = probs
            .iter()
            .enumerate()
            .map(|(i, &p)| (i, p))
            .collect();
        
        indexed_probs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let top_emotions: Vec<EmotionResult> = indexed_probs
            .iter()
            .take(3)
            .filter(|(_, score)| *score > 0.1) // Only include emotions with >10% confidence
            .map(|(idx, score)| EmotionResult {
                emotion: EMOTION_LABELS[*idx].to_string(),
                score: *score,
            })
            .collect();
        
        Ok(top_emotions)
    }
}

/// Map emotion to HSL hue value (0-360 degrees)
pub fn emotion_to_hue(emotion: &str) -> f32 {
    match emotion {
        // Yellow/Gold - Joy, excitement, amusement
        "joy" | "amusement" | "excitement" | "pride" => 50.0,
        
        // Pink/Magenta - Love, caring, gratitude
        "love" | "caring" | "gratitude" | "admiration" => 340.0,
        
        // Red - Anger, annoyance, disapproval
        "anger" | "annoyance" | "disapproval" | "disgust" => 0.0,
        
        // Violet - Fear, nervousness, embarrassment
        "fear" | "nervousness" | "embarrassment" => 270.0,
        
        // Blue - Sadness, grief, disappointment, remorse
        "sadness" | "grief" | "disappointment" | "remorse" => 220.0,
        
        // Orange - Surprise, curiosity, realization
        "surprise" | "curiosity" | "realization" | "confusion" => 30.0,
        
        // Green - Approval, optimism, relief, desire
        "approval" | "optimism" | "relief" | "desire" => 140.0,
        
        // Neutral/Gray
        "neutral" => 0.0,
        
        // Default
        _ => 0.0,
    }
}

/// Calculate weighted sentiment from emotion results
/// Returns a value between -1.0 and 1.0
pub fn emotions_to_sentiment(emotions: &[EmotionResult]) -> f32 {
    if emotions.is_empty() {
        return 0.0;
    }
    
    let mut sentiment = 0.0;
    let mut total_weight = 0.0;
    
    for result in emotions {
        let weight = result.score;
        let emotion_sentiment = match result.emotion.as_str() {
            // Positive emotions
            "joy" | "amusement" | "excitement" | "love" | "caring" | 
            "gratitude" | "admiration" | "approval" | "optimism" | 
            "relief" | "pride" | "desire" => 1.0,
            
            // Negative emotions
            "anger" | "annoyance" | "disapproval" | "disgust" | 
            "fear" | "nervousness" | "sadness" | "grief" | 
            "disappointment" | "remorse" | "embarrassment" => -1.0,
            
            // Neutral/ambiguous
            _ => 0.0,
        };
        
        sentiment += emotion_sentiment * weight;
        total_weight += weight;
    }
    
    if total_weight > 0.0 {
        (sentiment / total_weight).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
