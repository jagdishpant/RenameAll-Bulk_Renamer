#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod ui;

use std::path::PathBuf;
use ui::RenameAllApp;

fn main() -> eframe::Result<()> {
    // Parse arguments: allow passing files or folders directly on launch
    let args: Vec<String> = std::env::args().skip(1).collect();
    let initial_paths: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();

    // Load RenameAll icon for window title bar and taskbar
    let icon_data = {
        let icon_bytes = include_bytes!("../assets/RenameAll.png");
        if let Ok(img) = image::load_from_memory(icon_bytes) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            })
        } else {
            None
        }
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("RenameAll - Bulk Renamer")
        .with_inner_size([1060.0, 690.0])
        .with_min_inner_size([760.0, 500.0])
        .with_drag_and_drop(true);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "RenameAll",
        native_options,
        Box::new(|_cc| Ok(Box::new(RenameAllApp::new(initial_paths)))),
    )
}
