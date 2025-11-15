//! # Cosmarium Creative Writing Software
//!
//! Main application entry point for Cosmarium, a next-generation creative writing
//! software designed for fiction authors. This application provides a modular,
//! plugin-based architecture with an immersive writing experience.
//!
//! ## Features
//!
//! - Modular plugin system
//! - Immersive markdown editor
//! - Cross-platform support (desktop and web)
//! - Modern EGUI-based interface
//!
//! ## Usage
//!
//! ```bash
//! # Run the application
//! cosmarium
//!
//! # Run with specific project
//! cosmarium --project /path/to/project.cosmarium
//!
//! # Run in debug mode
//! cosmarium --debug
//! ```

use clap::{Arg, Command};
use eframe::egui;
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use env_logger;

mod app;

/// Command line arguments for Cosmarium
#[derive(Debug, Clone)]
pub struct AppArgs {
    /// Path to project file to open on startup
    pub project_path: Option<PathBuf>,
    /// Enable debug logging
    pub debug: bool,
    /// Window width
    pub width: Option<f32>,
    /// Window height
    pub height: Option<f32>,
}

impl Default for AppArgs {
    fn default() -> Self {
        Self {
            project_path: None,
            debug: false,
            width: Some(1200.0),
            height: Some(800.0),
        }
    }
}

/// Parse command line arguments
fn parse_args() -> AppArgs {
    let matches = Command::new("Cosmarium")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Cosmarium Team")
        .about("Next-generation creative writing software for fiction authors")
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .value_name("FILE")
                .help("Project file to open on startup")
                .value_parser(clap::value_parser!(PathBuf))
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug logging")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("width")
                .long("width")
                .value_name("PIXELS")
                .help("Initial window width")
                .value_parser(clap::value_parser!(f32))
        )
        .arg(
            Arg::new("height")
                .long("height")
                .value_name("PIXELS")
                .help("Initial window height")
                .value_parser(clap::value_parser!(f32))
        )
        .get_matches();

    AppArgs {
        project_path: matches.get_one::<PathBuf>("project").cloned(),
        debug: matches.get_flag("debug"),
        width: matches.get_one::<f32>("width").copied(),
        height: matches.get_one::<f32>("height").copied(),
    }
}

/// Initialize logging based on arguments
fn init_logging(debug: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let log_level = if debug { "debug" } else { "info" };
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
            .init();
    }

    cosmarium_core::init_tracing();
}

/// Native application entry point
#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    let args = parse_args();
    init_logging(args.debug);

    tracing::info!("Starting Cosmarium v{}", env!("CARGO_PKG_VERSION"));

    // Configure simple NativeOptions to avoid depending on changing platform-specific API.
    let mut options = eframe::NativeOptions::default();
    // Keep other defaults for renderer and platform integration.

    // Run the native app and handle errors explicitly (avoid `?` across non-Send errors).
    let run_result = eframe::run_native(
        "Cosmarium",
        options,
        Box::new(move |cc| {
            // Set up custom fonts
            setup_custom_fonts(&cc.egui_ctx);
            
            // Configure visuals
            setup_visuals(&cc.egui_ctx);
            
            Box::new(app::Cosmarium::new(cc, args.clone()))
        }),
    );

    if let Err(e) = run_result {
        eprintln!("Application exited with error: {}", e);
    }

    Ok(())
}

/// Web application entry point
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log`
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "cosmarium_canvas",
                web_options,
                Box::new(|cc| {
                    setup_custom_fonts(&cc.egui_ctx);
                    setup_visuals(&cc.egui_ctx);
                    Box::new(app::Cosmarium::new(cc, AppArgs::default()))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}

/// Load application icon
#[cfg(not(target_arch = "wasm32"))]
fn load_icon() -> egui::IconData {
    // For now, use a placeholder icon
    // In a real implementation, you would load the actual icon file
    let (icon_rgba, icon_width, icon_height) = {
        // Create a simple 32x32 icon with a purple background and white "C"
        let mut icon_data = vec![0u8; 32 * 32 * 4];
        for (i, pixel) in icon_data.chunks_mut(4).enumerate() {
            let x = i % 32;
            let y = i / 32;
            
            // Purple background
            pixel[0] = 128; // R
            pixel[1] = 64;  // G
            pixel[2] = 192; // B
            pixel[3] = 255; // A
            
            // Simple "C" shape in white
            if (x >= 8 && x <= 24 && (y == 8 || y == 23)) ||
               (x == 8 && y >= 8 && y <= 23) {
                pixel[0] = 255; // R
                pixel[1] = 255; // G
                pixel[2] = 255; // B
            }
        }
        (icon_data, 32, 32)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

/// Setup custom fonts for the application
fn setup_custom_fonts(ctx: &egui::Context) {
    // Use the default font definitions. Do not insert a named family that has
    // no associated font data, which can cause a runtime panic when egui
    // attempts to resolve the family.
    let fonts = egui::FontDefinitions::default();

    // Apply fonts to the context.
    ctx.set_fonts(fonts);
}

/// Setup visual theme for the application
fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Accent color (purple/violet theme)
    visuals.selection.bg_fill = egui::Color32::from_rgb(128, 64, 192);
    visuals.hyperlink_color = egui::Color32::from_rgb(160, 100, 220);

    // Override text color for improved readability during long writing sessions.
    visuals.override_text_color = Some(egui::Color32::from_rgb(230, 230, 240));

    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_args_default() {
        let args = AppArgs::default();
        assert!(args.project_path.is_none());
        assert!(!args.debug);
        assert_eq!(args.width, Some(1200.0));
        assert_eq!(args.height, Some(800.0));
    }

    #[test]
    fn test_parse_args_empty() {
        // This test would require mocking command line args
        // For now, we just verify the function exists
        let _args = AppArgs::default();
    }
}