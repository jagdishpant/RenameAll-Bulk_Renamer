use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;
use egui::{vec2, Align, Color32, Layout, RichText, ScrollArea, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::engine::{
    execute_batch_rename, execute_undo, update_preview_list, EnumeratePosition, FileItem,
    ItemStatus, RenameConfig, TargetScope, TextFormat, UndoEntry,
};

static NEXT_ITEM_ID: AtomicUsize = AtomicUsize::new(1);

/// Helper function to create a light gray input box with dark, crisp text
fn light_gray_input(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    desired_width: f32,
) -> egui::Response {
    let bg_color = Color32::from_rgb(238, 240, 244);
    let border_color = Color32::from_rgb(200, 205, 214);
    let text_color = Color32::from_rgb(20, 22, 28);
    let hint_color = Color32::from_rgb(115, 120, 130);

    egui::Frame::none()
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, border_color))
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .hint_text(RichText::new(hint).color(hint_color).size(13.0))
                    .text_color(text_color)
                    .frame(false)
                    .desired_width(desired_width),
            )
        })
        .inner
}

pub struct RenameAllApp {
    config: RenameConfig,
    items: Vec<FileItem>,
    undo_stack: Vec<Vec<UndoEntry>>,
    filter_query: String,
    status_message: Option<(String, bool)>, // (message, is_error)
    regex_error: Option<String>,
    is_dirty: bool,
    shell_registered: bool,
    app_icon_texture: Option<egui::TextureHandle>,
}

impl RenameAllApp {
    pub fn new(initial_paths: Vec<PathBuf>) -> Self {
        let mut app = Self {
            config: RenameConfig::default(),
            items: Vec::new(),
            undo_stack: Vec::new(),
            filter_query: String::new(),
            status_message: None,
            regex_error: None,
            is_dirty: true,
            shell_registered: check_shell_registered(),
            app_icon_texture: None,
        };

        if !initial_paths.is_empty() {
            app.add_paths(initial_paths);
        }

        app
    }

    pub fn add_paths(&mut self, paths: Vec<PathBuf>) {
        let mut new_items = Vec::new();

        for path in paths {
            if path.is_file() {
                if self.config.include_files && !self.items.iter().any(|i| i.original_path == path) {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
                        new_items.push(FileItem {
                            id: NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
                            original_path: path.clone(),
                            directory: parent,
                            original_name: file_name.to_string(),
                            new_name: file_name.to_string(),
                            is_directory: false,
                            selected: true,
                            status: ItemStatus::Unchanged,
                        });
                    }
                }
            } else if path.is_dir() {
                if self.config.include_folders && !self.items.iter().any(|i| i.original_path == path) {
                    if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                        let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
                        new_items.push(FileItem {
                            id: NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
                            original_path: path.clone(),
                            directory: parent,
                            original_name: folder_name.to_string(),
                            new_name: folder_name.to_string(),
                            is_directory: true,
                            selected: true,
                            status: ItemStatus::Unchanged,
                        });
                    }
                }

                // Scan children
                let max_depth = if self.config.include_subfolders { 50 } else { 1 };
                for entry in WalkDir::new(&path)
                    .min_depth(1)
                    .max_depth(max_depth)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path().to_path_buf();
                    let is_dir = entry.file_type().is_dir();

                    if is_dir && !self.config.include_folders {
                        continue;
                    }
                    if !is_dir && !self.config.include_files {
                        continue;
                    }
                    if self.items.iter().any(|i| i.original_path == entry_path) {
                        continue;
                    }

                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) {
                        let parent = entry_path.parent().unwrap_or(Path::new("")).to_path_buf();
                        new_items.push(FileItem {
                            id: NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
                            original_path: entry_path,
                            directory: parent,
                            original_name: name.clone(),
                            new_name: name,
                            is_directory: is_dir,
                            selected: true,
                            status: ItemStatus::Unchanged,
                        });
                    }
                }
            }
        }

        self.items.extend(new_items);
        self.is_dirty = true;
    }

    fn recalculate_preview(&mut self) {
        match update_preview_list(&mut self.items, &self.config) {
            Ok(()) => {
                self.regex_error = None;
            }
            Err(err) => {
                self.regex_error = Some(err);
            }
        }
        self.is_dirty = false;
    }

    fn apply_renaming(&mut self) {
        let (undo_entries, errors) = execute_batch_rename(&mut self.items);
        let renamed_count = undo_entries.len();

        if !undo_entries.is_empty() {
            self.undo_stack.push(undo_entries);
        }

        if errors.is_empty() {
            self.status_message = Some((
                format!("Successfully renamed {} item(s)!", renamed_count),
                false,
            ));
        } else {
            self.status_message = Some((
                format!(
                    "Renamed {} item(s). {} error(s) occurred:\n{}",
                    renamed_count,
                    errors.len(),
                    errors.join("\n")
                ),
                true,
            ));
        }

        self.is_dirty = true;
    }

    fn apply_undo(&mut self) {
        if let Some(last_undo) = self.undo_stack.pop() {
            let (reverted, errors) = execute_undo(last_undo);
            if errors.is_empty() {
                self.status_message = Some((
                    format!("Reverted {} item(s) back to original names.", reverted),
                    false,
                ));
            } else {
                self.status_message = Some((
                    format!(
                        "Reverted {} items, with {} error(s):\n{}",
                        reverted,
                        errors.len(),
                        errors.join("\n")
                    ),
                    true,
                ));
            }
            // Refresh items list
            self.is_dirty = true;
        }
    }
}

impl eframe::App for RenameAllApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load embedded RenameAll icon texture for in-app display
        if self.app_icon_texture.is_none() {
            let icon_bytes = include_bytes!("../../assets/RenameAll.png");
            if let Ok(img) = image::load_from_memory(icon_bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                self.app_icon_texture = Some(ctx.load_texture("rename_all_icon", color_image, egui::TextureOptions::LINEAR));
            }
        }

        // Handle drag & drop files onto the window
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|df| df.path.clone())
                    .collect();
                if !paths.is_empty() {
                    self.add_paths(paths);
                }
            }
        });

        // Recalculate preview if state changed
        if self.is_dirty {
            self.recalculate_preview();
        }

        // Top Command Bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if let Some(tex) = &self.app_icon_texture {
                    ui.image((tex.id(), vec2(26.0, 26.0)));
                    ui.add_space(2.0);
                }
                ui.heading(RichText::new("RenameAll").strong().size(19.0));
                ui.label(RichText::new("Bulk Renamer").size(13.0).color(Color32::from_rgb(130, 140, 155)));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);

                    // Context Menu Toggle Button
                    let ctx_label = if self.shell_registered {
                        "✓ In Explorer Menu"
                    } else {
                        "+ Add to Explorer Menu"
                    };
                    if ui.button(RichText::new(ctx_label).size(12.0)).clicked() {
                        if self.shell_registered {
                            unregister_shell_menu();
                            self.shell_registered = false;
                            self.status_message = Some(("Removed from Explorer context menu".into(), false));
                        } else {
                            register_shell_menu();
                            self.shell_registered = true;
                            self.status_message = Some(("Added to Windows Explorer context menu!".into(), false));
                        }
                    }

                    if ui
                        .add_enabled(
                            !self.undo_stack.is_empty(),
                            egui::Button::new(RichText::new("↺ Undo Last Rename").size(12.0)),
                        )
                        .clicked()
                    {
                        self.apply_undo();
                    }

                    if ui.button(RichText::new("🧹 Clear List").size(12.0)).clicked() {
                        self.items.clear();
                        self.is_dirty = true;
                        self.status_message = None;
                    }

                    if ui.button(RichText::new("📂 Add Folder...").size(12.0)).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.add_paths(vec![folder]);
                        }
                    }

                    if ui.button(RichText::new("📄 Add Files...").size(12.0)).clicked() {
                        if let Some(files) = rfd::FileDialog::new().pick_files() {
                            self.add_paths(files);
                        }
                    }
                });
            });
            ui.add_space(6.0);
        });

        // Bottom Action Bar
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ready_count = self
                    .items
                    .iter()
                    .filter(|i| i.selected && i.status.is_ready())
                    .count();
                let conflict_count = self
                    .items
                    .iter()
                    .filter(|i| i.selected && matches!(i.status, ItemStatus::Conflict(_)))
                    .count();
                let total_selected = self.items.iter().filter(|i| i.selected).count();

                // Status message or count summary
                if let Some((msg, is_err)) = &self.status_message {
                    let color = if *is_err {
                        Color32::from_rgb(230, 80, 80)
                    } else {
                        Color32::from_rgb(70, 190, 110)
                    };
                    ui.label(RichText::new(msg).color(color).strong());
                } else {
                    let text = format!(
                        "Selected: {} / {} total | Ready to rename: {} | Collisions: {}",
                        total_selected,
                        self.items.len(),
                        ready_count,
                        conflict_count
                    );
                    ui.label(RichText::new(text).weak());
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let can_rename = ready_count > 0 && conflict_count == 0;
                    let btn_text = format!("🚀 Apply Rename ({})", ready_count);
                    let button = egui::Button::new(RichText::new(btn_text).strong().size(14.0))
                        .min_size(vec2(160.0, 32.0));

                    if ui.add_enabled(can_rename, button).clicked() {
                        self.apply_renaming();
                    }
                });
            });
            ui.add_space(8.0);
        });

        // Left Configuration Sidebar
        egui::SidePanel::left("options_panel")
            .resizable(true)
            .default_width(320.0)
            .width_range(280.0..=480.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(10.0);
                    ui.heading(RichText::new("Search & Replace").size(15.0));

                    // Search input with light gray background
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Search:").strong());
                    });
                    let search_resp = light_gray_input(
                        ui,
                        &mut self.config.search,
                        "Search text or pattern...",
                        f32::INFINITY,
                    );
                    if search_resp.changed() {
                        self.is_dirty = true;
                    }

                    if let Some(err) = &self.regex_error {
                        ui.label(RichText::new(format!("⚠ {}", err)).color(Color32::from_rgb(235, 90, 90)).size(11.0));
                    }

                    // Replace input with light gray background
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Replace with:").strong());
                    });
                    let replace_resp = light_gray_input(
                        ui,
                        &mut self.config.replace,
                        "Replacement or tokens (e.g. $1, ${count}, ${YYYY})...",
                        f32::INFINITY,
                    );
                    if replace_resp.changed() {
                        self.is_dirty = true;
                    }

                    // Tokens helper info
                    ui.collapsing("Available Tokens (${count}, date)", |ui| {
                        ui.label(RichText::new("• ${count} : Sequential file number\n• ${YYYY} / ${YY} : 4-digit / 2-digit Year\n• ${MM} / ${DD} : Month / Day\n• ${hh} / ${mm} / ${ss} : Hour / Min / Sec\n• $1, $2 : Regex capture groups").size(11.0).weak());
                    });

                    ui.separator();

                    // Pattern Options
                    ui.label(RichText::new("Search Options").strong());
                    if ui.checkbox(&mut self.config.use_regex, "Use Regular Expressions").changed() {
                        self.is_dirty = true;
                    }
                    if ui.checkbox(&mut self.config.match_all, "Match All Occurrences (Global)").changed() {
                        self.is_dirty = true;
                    }
                    if ui.checkbox(&mut self.config.case_sensitive, "Case Sensitive").changed() {
                        self.is_dirty = true;
                    }

                    ui.separator();

                    // Target Scope
                    ui.label(RichText::new("Apply To").strong());
                    egui::ComboBox::from_id_salt("scope_combo")
                        .selected_text(self.config.scope.label())
                        .show_ui(ui, |ui| {
                            for scope in [
                                TargetScope::FilenameOnly,
                                TargetScope::FilenameAndExtension,
                                TargetScope::ExtensionOnly,
                            ] {
                                if ui
                                    .selectable_value(&mut self.config.scope, scope, scope.label())
                                    .changed()
                                {
                                    self.is_dirty = true;
                                }
                            }
                        });

                    ui.separator();

                    // Text Formatting
                    ui.label(RichText::new("Text Formatting (Case)").strong());
                    egui::ComboBox::from_id_salt("format_combo")
                        .selected_text(self.config.format.label())
                        .show_ui(ui, |ui| {
                            for fmt in [
                                TextFormat::None,
                                TextFormat::Lowercase,
                                TextFormat::Uppercase,
                                TextFormat::TitleCase,
                                TextFormat::CapitalizeFirst,
                            ] {
                                if ui.selectable_value(&mut self.config.format, fmt, fmt.label()).changed() {
                                    self.is_dirty = true;
                                }
                            }
                        });

                    ui.separator();

                    // Item Enumeration / Numbering
                    ui.label(RichText::new("Sequential Numbering").strong());
                    if ui.checkbox(&mut self.config.enumerate.enabled, "Enable Item Counter").changed() {
                        self.is_dirty = true;
                    }

                    if self.config.enumerate.enabled {
                        ui.horizontal(|ui| {
                            ui.label("Position:");
                            for pos in [
                                EnumeratePosition::Suffix,
                                EnumeratePosition::Prefix,
                                EnumeratePosition::None,
                            ] {
                                if ui
                                    .selectable_value(&mut self.config.enumerate.position, pos, pos.label())
                                    .changed()
                                {
                                    self.is_dirty = true;
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            if ui
                                .add(egui::DragValue::new(&mut self.config.enumerate.start).range(0..=999999))
                                .changed()
                            {
                                self.is_dirty = true;
                            }

                            ui.label("Step:");
                            if ui
                                .add(egui::DragValue::new(&mut self.config.enumerate.step).range(1..=100))
                                .changed()
                            {
                                self.is_dirty = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Padding digits:");
                            if ui
                                .add(egui::DragValue::new(&mut self.config.enumerate.padding).range(1..=10))
                                .changed()
                            {
                                self.is_dirty = true;
                            }

                            ui.label("Separator:");
                            if ui.add(egui::TextEdit::singleline(&mut self.config.enumerate.separator).desired_width(30.0)).changed() {
                                self.is_dirty = true;
                            }
                        });
                    }

                    ui.separator();

                    // Inclusion Filters
                    ui.label(RichText::new("Inclusion Filters").strong());
                    if ui.checkbox(&mut self.config.include_files, "Include Files").changed() {
                        self.is_dirty = true;
                    }
                    if ui.checkbox(&mut self.config.include_folders, "Include Folders").changed() {
                        self.is_dirty = true;
                    }
                    if ui.checkbox(&mut self.config.include_subfolders, "Scan Subfolders").changed() {
                        self.is_dirty = true;
                    }

                    ui.add_space(20.0);
                });
            });

        // Center Main Preview Table
        egui::CentralPanel::default().show(ctx, |ui| {
            // Preview Filter & Selection Bar
            ui.horizontal(|ui| {
                ui.label(RichText::new("Filter preview:").strong());
                let filter_resp = light_gray_input(
                    ui,
                    &mut self.filter_query,
                    "Type to filter list...",
                    220.0,
                );
                if filter_resp.changed() {
                    // Filter query changed
                }

                if !self.filter_query.is_empty() && ui.button("✖").clicked() {
                    self.filter_query.clear();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(RichText::new("Invert Selection").size(11.0)).clicked() {
                        for item in &mut self.items {
                            item.selected = !item.selected;
                        }
                        self.is_dirty = true;
                    }
                    if ui.button(RichText::new("Deselect All").size(11.0)).clicked() {
                        for item in &mut self.items {
                            item.selected = false;
                        }
                        self.is_dirty = true;
                    }
                    if ui.button(RichText::new("Select All").size(11.0)).clicked() {
                        for item in &mut self.items {
                            item.selected = true;
                        }
                        self.is_dirty = true;
                    }
                });
            });

            ui.add_space(4.0);

            if self.items.is_empty() {
                // Drag & Drop Empty State
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    if let Some(tex) = &self.app_icon_texture {
                        ui.image((tex.id(), vec2(64.0, 64.0)));
                        ui.add_space(10.0);
                    }
                    ui.label(RichText::new("Drag and drop files or folders here").size(20.0).weak());
                    ui.add_space(6.0);
                    ui.label(RichText::new("RenameAll • Bulk Renamer").size(13.0).color(Color32::from_rgb(130, 140, 155)));
                    ui.add_space(18.0);
                    if ui.button(RichText::new("📁 Browse Files...").size(14.0)).clicked() {
                        if let Some(files) = rfd::FileDialog::new().pick_files() {
                            self.add_paths(files);
                        }
                    }
                });
            } else {
                // Virtualized Table for maximum speed and smoothness
                let filter = self.filter_query.to_lowercase();
                let filtered_indices: Vec<usize> = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| {
                        if filter.is_empty() {
                            true
                        } else {
                            item.original_name.to_lowercase().contains(&filter)
                                || item.new_name.to_lowercase().contains(&filter)
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();

                let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 6.0;

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(Column::exact(36.0)) // Checkbox
                    .column(Column::initial(220.0).at_least(120.0)) // Original Name
                    .column(Column::initial(240.0).at_least(120.0)) // New Name
                    .column(Column::exact(100.0)) // Status badge
                    .column(Column::remainder().at_least(140.0)) // Folder Path
                    .header(24.0, |mut header| {
                        header.col(|ui| {
                            ui.label("");
                        });
                        header.col(|ui| {
                            ui.label(RichText::new("Original Name").strong());
                        });
                        header.col(|ui| {
                            ui.label(RichText::new("New Name (Preview)").strong());
                        });
                        header.col(|ui| {
                            ui.label(RichText::new("Status").strong());
                        });
                        header.col(|ui| {
                            ui.label(RichText::new("Folder").strong());
                        });
                    })
                    .body(|body| {
                        body.rows(text_height, filtered_indices.len(), |mut row| {
                            let row_idx = row.index();
                            let item_idx = filtered_indices[row_idx];
                            let item = &mut self.items[item_idx];

                            // Checkbox
                            row.col(|ui| {
                                if ui.checkbox(&mut item.selected, "").changed() {
                                    self.is_dirty = true;
                                }
                            });

                            // Original Name
                            row.col(|ui| {
                                let icon = if item.is_directory { "📁 " } else { "📄 " };
                                ui.label(format!("{}{}", icon, item.original_name));
                            });

                            // New Name Preview
                            row.col(|ui| {
                                if item.status.is_ready() {
                                    ui.label(RichText::new(&item.new_name).color(Color32::from_rgb(70, 180, 100)).strong());
                                } else {
                                    ui.label(&item.new_name);
                                }
                            });

                            // Status badge
                            row.col(|ui| {
                                match &item.status {
                                    ItemStatus::Ready => {
                                        ui.label(RichText::new("Ready").color(Color32::from_rgb(70, 180, 100)));
                                    }
                                    ItemStatus::Unchanged => {
                                        ui.label(RichText::new("Unchanged").weak());
                                    }
                                    ItemStatus::Conflict(msg) => {
                                        ui.label(RichText::new("Conflict").color(Color32::from_rgb(230, 80, 80)).strong())
                                            .on_hover_text(msg);
                                    }
                                    ItemStatus::Invalid(msg) => {
                                        ui.label(RichText::new("Invalid").color(Color32::from_rgb(230, 150, 40)).strong())
                                            .on_hover_text(msg);
                                    }
                                }
                            });

                            // Folder Path
                            row.col(|ui| {
                                ui.label(RichText::new(item.directory.to_string_lossy()).weak().size(11.0));
                            });
                        });
                    });
            }
        });
    }
}

// Windows Explorer Shell Context Menu Registration Helpers for RenameAll
fn check_shell_registered() -> bool {
    let output = std::process::Command::new("reg")
        .args(["query", "HKCU\\Software\\Classes\\*\\shell\\RenameAll"])
        .output();
    output.map(|o| o.status.success()).unwrap_or(false)
}

fn register_shell_menu() {
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_str = exe_path.to_string_lossy();
        let cmd = format!("\"{}\" \"%1\"", exe_str);
        let icon_spec = format!("\"{}\",0", exe_str);

        // Context Menu for Files
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/ve", "/d", "RenameAll (Bulk Renamer)", "/f"])
            .output();
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/v", "Icon", "/d", &icon_spec, "/f"])
            .output();
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll\\command", "/ve", "/d", &cmd, "/f"])
            .output();

        // Context Menu for Folders
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/ve", "/d", "RenameAll (Bulk Renamer)", "/f"])
            .output();
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/v", "Icon", "/d", &icon_spec, "/f"])
            .output();
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll\\command", "/ve", "/d", &cmd, "/f"])
            .output();

        // Context Menu for Directory Background
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll", "/ve", "/d", "RenameAll (Bulk Renamer)", "/f"])
            .output();
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll", "/v", "Icon", "/d", &icon_spec, "/f"])
            .output();
        let _ = std::process::Command::new("reg")
            .args(["add", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll\\command", "/ve", "/d", &cmd, "/f"])
            .output();
    }
}

fn unregister_shell_menu() {
    let _ = std::process::Command::new("reg")
        .args(["delete", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/f"])
        .output();
    let _ = std::process::Command::new("reg")
        .args(["delete", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/f"])
        .output();
    let _ = std::process::Command::new("reg")
        .args(["delete", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll", "/f"])
        .output();
}
