#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod engine;
mod ui;
pub mod installer;
pub mod single_instance;

use std::path::PathBuf;
use ui::RenameAllApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Check for CLI installation flags
    if args.iter().any(|a| a == "--install" || a == "-i" || a == "/install") {
        match installer::install_to_default_location(true, true, true) {
            Ok(path) => {
                println!("Successfully installed RenameAll to: {}", path.display());
                let _ = std::process::Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "[System.Windows.Forms.MessageBox]::Show('RenameAll was successfully installed to:\\n{}\\n\\nDesktop shortcut, Start Menu entry, and Windows Explorer context menu have been registered.', 'RenameAll Setup', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information)",
                            path.display().to_string().replace('\\', "\\\\")
                        ),
                    ])
                    .output();
            }
            Err(e) => {
                eprintln!("Installation error: {}", e);
                let _ = std::process::Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "[System.Windows.Forms.MessageBox]::Show('Installation failed: {}', 'RenameAll Setup Error', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Error)",
                            e.replace('\'', "''")
                        ),
                    ])
                    .output();
            }
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--uninstall" || a == "-u" || a == "/uninstall") {
        installer::uninstall();
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[System.Windows.Forms.MessageBox]::Show('RenameAll shortcuts and context menu entries have been removed.', 'RenameAll Uninstalled', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information)",
            ])
            .output();
        return Ok(());
    }

    // Filter out flags, remaining are file/folder paths
    let initial_paths: Vec<PathBuf> = args
        .into_iter()
        .filter(|a| !a.starts_with('-') && !a.starts_with('/'))
        .map(PathBuf::from)
        .collect();

    // Single-instance coordination:
    // If an instance is already running (e.g. from Explorer multi-file select),
    // send files to the primary instance over a named pipe and exit immediately.
    let ipc_receiver = match single_instance::check_single_instance(&initial_paths) {
        single_instance::InstanceRole::Secondary => {
            return Ok(());
        }
        single_instance::InstanceRole::Primary(rx) => rx,
    };

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
        .with_title(&format!("RenameAll v{} - Bulk Renamer", env!("CARGO_PKG_VERSION")))
        .with_inner_size([1160.0, 700.0])
        // Start at the upper-left corner of the primary monitor.
        .with_position([0.0, 0.0])
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
        Box::new(|_cc| Ok(Box::new(RenameAllApp::new(initial_paths, Some(ipc_receiver))))),
    )
}
