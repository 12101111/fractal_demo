#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] //Hide console window in release builds on Windows, this blocks stdout.

use tracing::Level;
use tracing_subscriber::{filter, prelude::*};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    tracing_subscriber::registry()
        //.with(console_subscriber::spawn())
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_thread_names(true),
        )
        .with(
            filter::Targets::new()
                .with_default(Level::DEBUG)
                .with_target("fractal_demo", Level::TRACE),
        )
        .init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Fractal Viewer",
        native_options,
        Box::new(|cc| Box::new(fractal_demo::FractalApp::new(cc))),
    );
}
