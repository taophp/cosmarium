use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{Write, Read};
use std::path::{Path, PathBuf};

const MODEL_URL: &str = "https://huggingface.co/SamLowe/roberta-base-go_emotions-onnx/resolve/main/onnx/model_quantized.onnx";
const TOKENIZER_URL: &str = "https://huggingface.co/SamLowe/roberta-base-go_emotions-onnx/resolve/main/tokenizer.json";

/// Get the cache directory for Cosmarium models
pub fn get_model_cache_dir() -> Result<PathBuf> {
    let cache_dir = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join("cosmarium").join("models")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        // Windows fallback
        PathBuf::from(userprofile).join(".cache").join("cosmarium").join("models")
    } else {
        PathBuf::from(".cosmarium_cache").join("models")
    };
    
    fs::create_dir_all(&cache_dir)
        .context("Failed to create model cache directory")?;
    
    Ok(cache_dir)
}

/// Download a file with progress indicator
fn download_with_progress(url: &str, dest: &Path, file_name: &str) -> Result<()> {
    tracing::info!("Downloading {} from {}", file_name, url);
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    
    let mut response = client.get(url).send()?;
    
    if !response.status().is_success() {
        anyhow::bail!("Failed to download {}: HTTP {}", file_name, response.status());
    }
    
    let total_size = response.content_length().unwrap_or(0);
    
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(format!("Downloading {}", file_name));
    
    let mut file = fs::File::create(dest)?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }
    
    pb.finish_with_message(format!("{} downloaded successfully", file_name));
    Ok(())
}

/// Ensure model and tokenizer are downloaded
pub fn ensure_model_downloaded() -> Result<(PathBuf, PathBuf)> {
    let cache_dir = get_model_cache_dir()?;
    let model_path = cache_dir.join("roberta_go_emotions_quantized.onnx");
    let tokenizer_path = cache_dir.join("tokenizer.json");
    
    // Download model if not exists
    if !model_path.exists() {
        tracing::info!("Model not found in cache, downloading...");
        download_with_progress(MODEL_URL, &model_path, "Emotion Detection Model (125 MB)")?;
    } else {
        tracing::info!("Model found in cache: {:?}", model_path);
    }
    
    // Download tokenizer if not exists
    if !tokenizer_path.exists() {
        tracing::info!("Tokenizer not found in cache, downloading...");
        download_with_progress(TOKENIZER_URL, &tokenizer_path, "Tokenizer Config")?;
    } else {
        tracing::info!("Tokenizer found in cache: {:?}", tokenizer_path);
    }
    
    Ok((model_path, tokenizer_path))
}
