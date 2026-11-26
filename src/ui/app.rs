use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use walkdir::WalkDir;
use egui::{vec2, Align, Align2, Color32, FontId, Layout, RichText, ScrollArea, Sense, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::engine::{
    execute_batch_rename, execute_undo, update_preview_list, EnumeratePosition, FileItem,
    ItemStatus, RenameConfig, TargetScope, TextFormat, UndoEntry,
};

static NEXT_ITEM_ID: AtomicUsize = AtomicUsize::new(1);

// Persist only the optional UI size preference (percentage), under the user profile.
// File I/O occurs at startup or when the user changes the size, not every frame.
fn ui_scale_settings_file() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("RenameAll").join("ui_scale.txt"))
}

fn read_ui_scale() -> f32 {
    ui_scale_settings_file()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|value| value.trim().parse::<u32>().ok())
        .map(|percent| ((percent.clamp(70, 120) as f32 / 10.0).round() * 10.0) / 100.0_f32)
        .unwrap_or(0.90_f32)
}

fn save_ui_scale(scale: f32) {
    if let Some(path) = ui_scale_settings_file() {
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let percent = (scale * 100.0_f32).round() as u32;
        let _ = std::fs::write(path, percent.to_string());
    }
}


/// Helper function to create a light gray input box with dark, crisp text
fn light_gray_input(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    desired_width: f32,
) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let bg_color = if dark { Color32::from_rgb(67, 45, 32) } else { Color32::from_rgb(255, 239, 222) };
    let border_color = if dark { Color32::from_rgb(127, 75, 40) } else { Color32::from_rgb(222, 166, 118) };
    let text_color = if dark { Color32::from_rgb(255, 237, 215) } else { Color32::from_rgb(55, 31, 20) };
    let hint_color = if dark { Color32::from_rgb(193, 155, 129) } else { Color32::from_rgb(136, 92, 66) };

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

// Paint a single-line cell in a clipped viewport. Its X position is controlled by
// the matching horizontal scrollbar beneath the table, not by per-row scrolling.
fn scrolling_cell(ui: &mut egui::Ui, text: &str, offset: f32, color: Color32, font: FontId) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), ui.available_height().max(16.0)),
        Sense::hover(),
    );
    let clip = ui.clip_rect().intersect(rect);
    ui.painter().with_clip_rect(clip).text(
        egui::pos2(rect.left() + 2.0 - offset, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    response
}


// Draw platform-independent line icons rather than relying on emoji glyph coverage.
fn action_button(ui: &mut egui::Ui, caption: &str, kind: &str) -> egui::Response {
    let response = ui.button(format!("      {}", caption));
    let r = response.rect;
    let x = r.left() + 12.0;
    let y = r.center().y;
    let color = ui.visuals().text_color();
    let stroke = Stroke::new(1.7_f32, color);
    let p = ui.painter();
    let pt = |dx: f32, dy: f32| egui::pos2(x + dx, y + dy);
    match kind {
        "folder" => {
            p.line_segment([pt(-5.0,-5.0),pt(-1.0,-5.0)],stroke);
            p.line_segment([pt(-1.0,-5.0),pt(1.0,-3.0)],stroke);
            p.rect_stroke(egui::Rect::from_min_max(pt(-6.0,-3.0),pt(6.0,5.0)),2.0,stroke);
        }
        "file" => {
            p.rect_stroke(egui::Rect::from_min_max(pt(-5.0,-6.0),pt(5.0,6.0)),1.0,stroke);
            p.line_segment([pt(-2.0,-1.0),pt(3.0,-1.0)],stroke);
            p.line_segment([pt(-2.0,2.0),pt(3.0,2.0)],stroke);
        }
        "trash" => {
            p.rect_stroke(egui::Rect::from_min_max(pt(-4.0,-3.0),pt(4.0,6.0)),1.0,stroke);
            p.line_segment([pt(-6.0,-5.0),pt(6.0,-5.0)],stroke);
            p.line_segment([pt(-2.0,-7.0),pt(2.0,-7.0)],stroke);
        }
        "undo" => {
            p.circle_stroke(pt(1.0,0.0),5.0,stroke);
            p.line_segment([pt(-5.0,-5.0),pt(-5.0,0.0)],stroke);
            p.line_segment([pt(-5.0,-5.0),pt(0.0,-5.0)],stroke);
        }
        "book" => {
            p.line_segment([pt(0.0,-5.0),pt(0.0,6.0)],stroke);
            p.line_segment([pt(-6.0,-5.0),pt(-6.0,5.0)],stroke);
            p.line_segment([pt(6.0,-5.0),pt(6.0,5.0)],stroke);
            p.line_segment([pt(-6.0,5.0),pt(0.0,6.0)],stroke);
            p.line_segment([pt(0.0,6.0),pt(6.0,5.0)],stroke);
            p.line_segment([pt(-6.0,-5.0),pt(0.0,-4.0)],stroke);
            p.line_segment([pt(0.0,-4.0),pt(6.0,-5.0)],stroke);
        }
        "settings" => {
            p.circle_stroke(pt(0.0,0.0),5.0,stroke);
            p.circle_stroke(pt(0.0,0.0),2.0,stroke);
            for (dx,dy) in [(0.0,-8.0),(0.0,8.0),(-8.0,0.0),(8.0,0.0)] {
                p.line_segment([pt(dx*0.65,dy*0.65),pt(dx,dy)],stroke);
            }
        }
        "apply" => {
            p.line_segment([pt(-6.0,0.0),pt(-1.0,5.0)],stroke);
            p.line_segment([pt(-1.0,5.0),pt(7.0,-5.0)],stroke);
        }
        _ => {}
    }
    response
}

fn sort_button(ui: &mut egui::Ui, title: &str, selected: bool, ascending: bool) -> bool {
    let response = ui.add(
        egui::Button::new(RichText::new(title).strong())
            .frame(false)
            .min_size(vec2(ui.available_width(), 25.0))
    );
    if selected {
        let cx = response.rect.right() - 9.0;
        let cy = response.rect.center().y;
        let points = if ascending {
            vec![egui::pos2(cx,cy-4.0),egui::pos2(cx-4.0,cy+3.0),egui::pos2(cx+4.0,cy+3.0)]
        } else {
            vec![egui::pos2(cx-4.0,cy-3.0),egui::pos2(cx+4.0,cy-3.0),egui::pos2(cx,cy+4.0)]
        };
        ui.painter().add(egui::Shape::convex_polygon(points, ui.visuals().text_color(), Stroke::NONE));
    }
    response.on_hover_text("Click to sort ascending or descending").clicked()
}

fn set_orange_theme(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    for (text_style, size) in [
        (egui::TextStyle::Small, 11.0),
        (egui::TextStyle::Body, 13.0),
        (egui::TextStyle::Button, 13.0),
        (egui::TextStyle::Heading, 19.0),
        (egui::TextStyle::Monospace, 12.0),
    ] {
        style.text_styles.insert(text_style, FontId::proportional(size));
    }
    style.spacing.button_padding = vec2(10.0, 5.0);
    style.spacing.item_spacing = vec2(6.0, 6.0);
    let mut visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    if dark {
        visuals.panel_fill = Color32::from_rgb(33, 25, 21);
        visuals.window_fill = Color32::from_rgb(44, 31, 24);
        visuals.extreme_bg_color = Color32::from_rgb(25, 19, 16);
        visuals.faint_bg_color = Color32::from_rgb(57, 39, 29);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(65, 43, 31);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(111, 57, 27);
        visuals.widgets.active.bg_fill = Color32::from_rgb(155, 72, 24);
        visuals.selection.bg_fill = Color32::from_rgb(170, 79, 27);
        visuals.hyperlink_color = Color32::from_rgb(255, 167, 91);
    } else {
        visuals.panel_fill = Color32::from_rgb(255, 247, 239);
        visuals.window_fill = Color32::from_rgb(255, 251, 247);
        visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);
        visuals.faint_bg_color = Color32::from_rgb(255, 237, 218);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(255, 232, 207);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(255, 209, 161);
        visuals.widgets.active.bg_fill = Color32::from_rgb(255, 174, 104);
        visuals.selection.bg_fill = Color32::from_rgb(227, 128, 49);
        visuals.hyperlink_color = Color32::from_rgb(155, 66, 18);
    }
    style.visuals = visuals;
    ctx.set_style(style);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    OriginalName,
    NewName,
    Status,
    Folder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

pub struct RenameAllApp {
    config: RenameConfig,
    items: Vec<FileItem>,
    source_paths: Vec<PathBuf>,
    undo_stack: Vec<Vec<UndoEntry>>,
    filter_query: String,
    status_message: Option<(String, bool)>, // (message, is_error)
    regex_error: Option<String>,
    is_dirty: bool,
    shell_registered: bool,
    app_icon_texture: Option<egui::TextureHandle>,
    ipc_receiver: Option<Receiver<Vec<PathBuf>>>,
    // Practical Recipe Guide Dialog State
    show_recipes_dialog: bool,
    // Installation Dialog State
    show_install_dialog: bool,
    install_opt_desktop: bool,
    install_opt_start_menu: bool,
    install_opt_context_menu: bool,
    // Sorting state
    sort_column: Option<SortColumn>,
    sort_direction: SortDirection,
    // Independently scrollable positions and visible widths of the four text columns.
    preview_scroll_x: [f32; 4],
    preview_column_widths: [f32; 4],
    preview_column_lefts: [f32; 4],
    theme_dark: bool,
    pending_initial_paths: Vec<PathBuf>,
    rendered_first_frame: bool,
    ui_scale: f32,
    slider_draft_percent: Option<i32>,
    initial_pixels_per_point: Option<f32>,
}

impl RenameAllApp {
    pub fn new(initial_paths: Vec<PathBuf>, ipc_receiver: Option<Receiver<Vec<PathBuf>>>) -> Self {
        let app = Self {
            config: RenameConfig::default(),
            items: Vec::new(),
            source_paths: Vec::new(),
            undo_stack: Vec::new(),
            filter_query: String::new(),
            status_message: None,
            regex_error: None,
            is_dirty: true,
            shell_registered: false,
            app_icon_texture: None,
            ipc_receiver,
            show_recipes_dialog: false,
            show_install_dialog: false,
            install_opt_desktop: crate::installer::is_desktop_shortcut_installed(),
            install_opt_start_menu: crate::installer::is_start_menu_shortcut_installed(),
            install_opt_context_menu: crate::installer::is_context_menu_installed(),
            sort_column: None,
            sort_direction: SortDirection::Ascending,
            preview_scroll_x: [0.0; 4],
            preview_column_widths: [240.0, 240.0, 100.0, 280.0],
            preview_column_lefts: [0.0; 4],
            theme_dark: true,
            pending_initial_paths: initial_paths,
            rendered_first_frame: false,
            ui_scale: read_ui_scale(),
            slider_draft_percent: None,
            initial_pixels_per_point: None,
        };


        app
    }

    fn change_ui_scale(&mut self, ctx: &egui::Context, requested: f32) {
        let scale = (requested.clamp(0.70_f32, 1.20_f32) * 10.0_f32).round() / 10.0_f32;
        if (self.ui_scale - scale).abs() > 0.001_f32 {
            self.ui_scale = scale;
            if let Some(native_ppp) = self.initial_pixels_per_point {
                ctx.set_pixels_per_point(native_ppp * scale);
            }
            save_ui_scale(scale);
            ctx.request_repaint();
        }
    }

    pub fn add_paths(&mut self, paths: Vec<PathBuf>) {
        for p in &paths {
            if !self.source_paths.contains(p) {
                self.source_paths.push(p.clone());
            }
        }

        let mut new_items = Vec::new();

        for path in paths {
            if path.is_file() {
                if !self.items.iter().any(|i| i.original_path == path) {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
                        new_items.push(FileItem {
                            id: NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
                            original_path: path.clone(),
                            directory: parent,
                            original_name: file_name.to_string(),
                            new_name: file_name.to_string(),
                            is_directory: false,
                            is_subfolder: false,
                            selected: self.config.include_files,
                            status: ItemStatus::Unchanged,
                        });
                    }
                }
            } else if path.is_dir() {
                if !self.items.iter().any(|i| i.original_path == path) {
                    if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                        let parent = path.parent().unwrap_or(Path::new("")).to_path_buf();
                        new_items.push(FileItem {
                            id: NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
                            original_path: path.clone(),
                            directory: parent,
                            original_name: folder_name.to_string(),
                            new_name: folder_name.to_string(),
                            is_directory: true,
                            is_subfolder: false,
                            selected: self.config.include_folders,
                            status: ItemStatus::Unchanged,
                        });
                    }
                }

                // Scan children and descendants
                for entry in WalkDir::new(&path)
                    .min_depth(1)
                    .max_depth(50)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path().to_path_buf();
                    let is_dir = entry.file_type().is_dir();
                    let is_subfolder = entry.depth() > 1;

                    if self.items.iter().any(|i| i.original_path == entry_path)
                        || new_items.iter().any(|i: &FileItem| i.original_path == entry_path)
                    {
                        continue;
                    }

                    let selected = if is_subfolder && !self.config.include_subfolders {
                        false
                    } else if is_dir {
                        self.config.include_folders
                    } else {
                        self.config.include_files
                    };

                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) {
                        let parent = entry_path.parent().unwrap_or(Path::new("")).to_path_buf();
                        new_items.push(FileItem {
                            id: NEXT_ITEM_ID.fetch_add(1, Ordering::Relaxed),
                            original_path: entry_path,
                            directory: parent,
                            original_name: name.clone(),
                            new_name: name,
                            is_directory: is_dir,
                            is_subfolder,
                            selected,
                            status: ItemStatus::Unchanged,
                        });
                    }
                }
            }
        }

        self.items.extend(new_items);
        if self.sort_column.is_some() {
            self.sort_items();
        }
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

    pub fn toggle_sort(&mut self, col: SortColumn) {
        if self.sort_column == Some(col) {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = Some(col);
            self.sort_direction = SortDirection::Ascending;
        }
        self.sort_items();
    }

    pub fn sort_items(&mut self) {
        let Some(col) = self.sort_column else { return };
        let asc = self.sort_direction == SortDirection::Ascending;

        self.items.sort_by(|a, b| {
            let cmp = match col {
                SortColumn::OriginalName => {
                    a.original_name.to_lowercase().cmp(&b.original_name.to_lowercase())
                        .then_with(|| a.original_name.cmp(&b.original_name))
                }
                SortColumn::NewName => {
                    a.new_name.to_lowercase().cmp(&b.new_name.to_lowercase())
                        .then_with(|| a.new_name.cmp(&b.new_name))
                }
                SortColumn::Status => {
                    let rank = |s: &ItemStatus| match s {
                        ItemStatus::Ready => 0,
                        ItemStatus::Conflict(_) => 1,
                        ItemStatus::Invalid(_) => 2,
                        ItemStatus::Unchanged => 3,
                    };
                    rank(&a.status).cmp(&rank(&b.status))
                        .then_with(|| a.original_name.to_lowercase().cmp(&b.original_name.to_lowercase()))
                }
                SortColumn::Folder => {
                    a.directory.cmp(&b.directory)
                        .then_with(|| a.original_name.to_lowercase().cmp(&b.original_name.to_lowercase()))
                }
            };

            if asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
        self.is_dirty = true;
    }
}

impl eframe::App for RenameAllApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Preserve the screen's native DPI scaling, then apply the user's
        // independent interface-size preference (text, buttons, spacing, icons).
        if self.initial_pixels_per_point.is_none() {
            let native_ppp = ctx.pixels_per_point();
            self.initial_pixels_per_point = Some(native_ppp);
            ctx.set_pixels_per_point(native_ppp * self.ui_scale);
        }
        // Apply app-wide typography and orange palette once on first frame.
        if !ctx.memory(|m| m.data.get_temp::<bool>(egui::Id::new("theme_initialized")).unwrap_or(false)) {
            set_orange_theme(ctx, self.theme_dark);
            ctx.memory_mut(|m| m.data.insert_temp(egui::Id::new("theme_initialized"), true));
        }
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

        // Draw an initial frame before scanning startup paths; this avoids
        // delaying the first visible window for recursive folder traversal.
        if self.rendered_first_frame && !self.pending_initial_paths.is_empty() {
            let paths = std::mem::take(&mut self.pending_initial_paths);
            self.add_paths(paths);
        }
        if !self.rendered_first_frame {
            self.rendered_first_frame = true;
            ctx.request_repaint();
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

        // Check for paths forwarded from secondary instances (e.g. Explorer multi-selection)
        if let Some(rx) = &self.ipc_receiver {
            let mut forwarded = Vec::new();
            while let Ok(paths) = rx.try_recv() {
                forwarded.extend(paths);
            }
            if !forwarded.is_empty() {
                self.add_paths(forwarded);
                ctx.request_repaint();
            }
        }

        // Recalculate preview if state changed
        if self.is_dirty {
            self.recalculate_preview();
        }

        // Top Command Bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if let Some(tex) = &self.app_icon_texture {
                    ui.image((tex.id(), vec2(24.0, 24.0)));
                    ui.add_space(2.0);
                }
                ui.heading(RichText::new("RenameAll").strong().size(19.0));
                ui.label(RichText::new(format!("Bulk Renamer  v{}", env!("CARGO_PKG_VERSION"))).size(13.0).color(Color32::from_rgb(130, 140, 155)));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.selectable_label(self.theme_dark, "Dark").clicked() {
                        self.theme_dark = true;
                        set_orange_theme(ctx, true);
                    }
                    if ui.selectable_label(!self.theme_dark, "Light").clicked() {
                        self.theme_dark = false;
                        set_orange_theme(ctx, false);
                    }

                    // Practical Recipe Guide Button
                    if action_button(ui, "Recipe Guide", "book").clicked() {
                        self.show_recipes_dialog = true;
                    }

                    // Setup & Integration Button
                    if action_button(ui, "Integration / Setup", "settings").clicked() {
                        self.install_opt_desktop = crate::installer::is_desktop_shortcut_installed();
                        self.install_opt_start_menu = crate::installer::is_start_menu_shortcut_installed();
                        self.install_opt_context_menu = crate::installer::is_context_menu_installed();
                        self.show_install_dialog = true;
                    }

                    let undo_clicked = ui.add_enabled_ui(!self.undo_stack.is_empty(), |ui| {
                        action_button(ui, "Undo Last Rename", "undo").clicked()
                    }).inner;
                    if undo_clicked {
                        self.apply_undo();
                    }

                    if action_button(ui, "Clear List", "trash").clicked() {
                        self.items.clear();
                        self.source_paths.clear();
                        self.is_dirty = true;
                        self.status_message = None;
                    }

                    if action_button(ui, "Add Folder...", "folder").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.add_paths(vec![folder]);
                        }
                    }

                    if action_button(ui, "Add Files...", "file").clicked() {
                        if let Some(files) = rfd::FileDialog::new().pick_files() {
                            self.add_paths(files);
                        }
                    }

                    // New display size control: exactly 70%-120%, in 10% increments.
                    // Placed immediately to the left of Add Files in this right-to-left toolbar.
                    ui.menu_button(
                        format!("Display size: {}%", (self.ui_scale * 100.0_f32).round() as u32),
                        |ui| {
                            ui.set_min_width(230.0);
                            ui.label(RichText::new("Display size").strong());
                            // Keep a draft while the thumb is being dragged. Updating the
                            // pixels-per-point during a drag would move the thumb under the
                            // pointer and cause a disruptive sequence of resizes.
                            let applied_percent = (self.ui_scale * 100.0_f32).round() as i32;
                            let mut percent = self.slider_draft_percent.unwrap_or(applied_percent);
                            let response = ui.add(
                                egui::Slider::new(&mut percent, 70..=120)
                                    .step_by(10.0)
                                    .suffix("%")
                            ).on_hover_text("Release the slider to apply the selected display size");
                            if response.drag_stopped() {
                                // Release takes priority: egui may also report dragged() on
                                // the final frame of the gesture.
                                self.slider_draft_percent = None;
                                self.change_ui_scale(ctx, percent as f32 / 100.0_f32);
                            } else if response.dragged() {
                                self.slider_draft_percent = Some(percent);
                            } else if response.changed() {
                                // Track clicks and keyboard adjustments apply immediately.
                                self.slider_draft_percent = None;
                                self.change_ui_scale(ctx, percent as f32 / 100.0_f32);
                            } else if self.slider_draft_percent.is_some() {
                                // Discard any interrupted drag (e.g. a dismissed menu).
                                self.slider_draft_percent = None;
                            }
                        },
                    );
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
                    let btn_text = format!("Apply Rename ({})", ready_count);
                    let button = egui::Button::new(RichText::new(btn_text).strong().size(12.0))
                        .min_size(vec2(170.0, 36.0));

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
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("Search & Replace").size(13.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .button(RichText::new(" Examples").size(11.0))
                                .on_hover_text("View Practical Recipe Examples from documentation")
                                .clicked()
                            {
                                self.show_recipes_dialog = true;
                            }
                        });
                    });

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
                        ui.label(RichText::new(format!("Warning: {}", err)).color(Color32::from_rgb(235, 90, 90)).size(11.0));
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
                        ui.label(RichText::new("- ${count} : Sequential file number\n- ${YYYY} / ${YY} : 4-digit / 2-digit Year\n- ${MM} / ${DD} : Month / Day\n- ${hh} / ${mm} / ${ss} : Hour / Min / Sec\n- $1, $2 : Regex capture groups").size(11.0).weak());
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
                        let inc = self.config.include_files;
                        let scan_sub = self.config.include_subfolders;
                        for item in &mut self.items {
                            if !item.is_directory {
                                if inc {
                                    if !item.is_subfolder || scan_sub {
                                        item.selected = true;
                                    }
                                } else {
                                    item.selected = false;
                                }
                            }
                        }
                        self.is_dirty = true;
                    }
                    if ui.checkbox(&mut self.config.include_folders, "Include Folders").changed() {
                        let inc = self.config.include_folders;
                        let scan_sub = self.config.include_subfolders;
                        for item in &mut self.items {
                            if item.is_directory {
                                if inc {
                                    if !item.is_subfolder || scan_sub {
                                        item.selected = true;
                                    }
                                } else {
                                    item.selected = false;
                                }
                            }
                        }
                        self.is_dirty = true;
                    }
                    if ui.checkbox(&mut self.config.include_subfolders, "Scan Subfolders").changed() {
                        let scan_sub = self.config.include_subfolders;
                        let inc_files = self.config.include_files;
                        let inc_folders = self.config.include_folders;
                        for item in &mut self.items {
                            if item.is_subfolder {
                                if scan_sub {
                                    if item.is_directory {
                                        item.selected = inc_folders;
                                    } else {
                                        item.selected = inc_files;
                                    }
                                } else {
                                    item.selected = false;
                                }
                            }
                        }
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

                if !self.filter_query.is_empty() && ui.button("Clear").clicked() {
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
                    ui.label(RichText::new("RenameAll - Bulk Renamer").size(13.0).color(Color32::from_rgb(130, 140, 155)));
                    ui.add_space(18.0);
                    if action_button(ui, "Browse Files...", "folder").clicked() {
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

                let text_height = egui::TextStyle::Body.resolve(ui.style()).size + 14.0;

                let row_area_height = (ui.available_height() - 22.0).max(60.0);
                ui.allocate_ui(vec2(ui.available_width(), row_area_height), |ui| {
                        TableBuilder::new(ui)
                            .id_salt("rename_all_table_builder")
                            .striped(true)
                            .resizable(true)
                            .cell_layout(Layout::left_to_right(Align::Center))
                            .column(Column::exact(36.0)) // Checkbox
                            .column(Column::initial(240.0).at_least(100.0).clip(true).resizable(true)) // Original Name
                            .column(Column::initial(240.0).at_least(100.0).clip(true).resizable(true)) // New Name
                            .column(Column::initial(100.0).at_least(75.0).clip(true).resizable(true)) // Status badge
                            .column(Column::initial(280.0).at_least(120.0).clip(true).resizable(true)) // Folder Path
                            .header(34.0, |mut header| {
                                header.col(|ui| {
                                    ui.label("");
                                });
                                header.col(|ui| {
                                    self.preview_column_lefts[0] = ui.max_rect().left();
                                    self.preview_column_widths[0] = ui.max_rect().width();
                                    if sort_button(ui, "Original Name", self.sort_column == Some(SortColumn::OriginalName), self.sort_direction == SortDirection::Ascending) {
                                        self.toggle_sort(SortColumn::OriginalName);
                                    }
                                });
                                header.col(|ui| {
                                    self.preview_column_lefts[1] = ui.max_rect().left();
                                    self.preview_column_widths[1] = ui.max_rect().width();
                                    if sort_button(ui, "New Name (Preview)", self.sort_column == Some(SortColumn::NewName), self.sort_direction == SortDirection::Ascending) {
                                        self.toggle_sort(SortColumn::NewName);
                                    }
                                });
                                header.col(|ui| {
                                    self.preview_column_lefts[2] = ui.max_rect().left();
                                    self.preview_column_widths[2] = ui.max_rect().width();
                                    if sort_button(ui, "Status", self.sort_column == Some(SortColumn::Status), self.sort_direction == SortDirection::Ascending) {
                                        self.toggle_sort(SortColumn::Status);
                                    }
                                });
                                header.col(|ui| {
                                    self.preview_column_lefts[3] = ui.max_rect().left();
                                    self.preview_column_widths[3] = ui.max_rect().width();
                                    if sort_button(ui, "Folder", self.sort_column == Some(SortColumn::Folder), self.sort_direction == SortDirection::Ascending) {
                                        self.toggle_sort(SortColumn::Folder);
                                    }
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
                                        let icon = if item.is_directory { "[DIR] " } else { "[FILE] " };
                                        let full_text = format!("{}{}", icon, item.original_name);
                                        scrolling_cell(ui, &full_text, self.preview_scroll_x[0],
                                            ui.visuals().text_color(), egui::TextStyle::Body.resolve(ui.style()))
                                            .on_hover_text(&item.original_name);
                                    });

                                    // New Name Preview
                                    row.col(|ui| {
                                        let color = if item.status.is_ready() {
                                            Color32::from_rgb(70, 180, 100)
                                        } else {
                                            ui.visuals().text_color()
                                        };
                                        scrolling_cell(ui, &item.new_name, self.preview_scroll_x[1],
                                            color, egui::TextStyle::Body.resolve(ui.style()))
                                            .on_hover_text(&item.new_name);
                                    });

                                    // Status badge
                                    row.col(|ui| {
                                        let (status_text, color, tooltip) = match &item.status {
                                            ItemStatus::Ready => ("Ready", Color32::from_rgb(70, 180, 100), None),
                                            ItemStatus::Unchanged => ("Unchanged", ui.visuals().weak_text_color(), None),
                                            ItemStatus::Conflict(msg) => ("Conflict", Color32::from_rgb(230, 80, 80), Some(msg.as_str())),
                                            ItemStatus::Invalid(msg) => ("Invalid", Color32::from_rgb(230, 150, 40), Some(msg.as_str())),
                                        };
                                        let response = scrolling_cell(ui, status_text, self.preview_scroll_x[2],
                                            color, egui::TextStyle::Body.resolve(ui.style()));
                                        if let Some(message) = tooltip {
                                            response.on_hover_text(message);
                                        }
                                    });

                                    // Folder Path
                                    row.col(|ui| {
                                        let folder_str = item.directory.to_string_lossy().to_string();
                                        scrolling_cell(ui, &folder_str, self.preview_scroll_x[3],
                                            ui.visuals().weak_text_color(), FontId::proportional(11.0))
                                            .on_hover_text(&folder_str);
                                    });
                                });
                            });
                    });

                // One scrollbar per text column, at the bottom of the file list.
                // The scroll range follows the widest entry, and the scroll offset is
                // applied to *all* visible rows in the corresponding column.
                let max_chars = filtered_indices.iter().fold([0usize; 4], |mut lens, &idx| {
                    let item = &self.items[idx];
                    lens[0] = lens[0].max(item.original_name.chars().count() + 3);
                    lens[1] = lens[1].max(item.new_name.chars().count());
                    lens[2] = lens[2].max(match item.status {
                        ItemStatus::Unchanged => 9,
                        ItemStatus::Conflict(_) => 8,
                        ItemStatus::Invalid(_) => 7,
                        ItemStatus::Ready => 5,
                    });
                    lens[3] = lens[3].max(item.directory.to_string_lossy().chars().count());
                    lens
                });
                let widths = self.preview_column_widths;
                // Reserve enough space for wide glyphs without measuring every row's galley.
                // A little extra range is harmless; the actual text remains fully visible.
                let full_widths = [
                    max_chars[0] as f32 * 12.0 + 8.0,
                    max_chars[1] as f32 * 12.0 + 8.0,
                    max_chars[2] as f32 * 9.0 + 8.0,
                    max_chars[3] as f32 * 9.0 + 8.0,
                ];
                // Align bars with actual table header cell screen rectangles, including
                // egui_extras padding and column separators (not guessed widths).
                let (_, bar_rect) = ui.allocate_space(vec2(ui.available_width(), 28.0));
                for col in 0..4 {
                    let width = widths[col].max(1.0);
                    let area = egui::Rect::from_min_size(
                        egui::pos2(self.preview_column_lefts[col], bar_rect.top()),
                        vec2(width, 26.0),
                    );
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area), |ui| {
                        ui.spacing_mut().scroll.bar_width = 16.0;
                        ui.spacing_mut().scroll.bar_inner_margin = 2.0;
                        ui.spacing_mut().scroll.bar_outer_margin = 2.0;
                        let response = ScrollArea::horizontal()
                            .id_salt(("preview_column_scroll", col))
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                            .auto_shrink([false, false])
                            .drag_to_scroll(false)
                            .show(ui, |ui| {
                                ui.set_min_width(full_widths[col].max(width));
                                ui.allocate_space(vec2(0.0, 1.0));
                            });
                        self.preview_scroll_x[col] = response.state.offset.x;
                    });
                }
            }
        });

        // Windows Setup & Integration Modal Dialog
        if self.show_install_dialog {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_install_dialog = false;
            }
            let mut is_open = true;
            egui::Window::new(" Windows Integration & Setup")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .min_width(420.0)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Install RenameAll and configure Windows shortcuts:").strong());
                    ui.add_space(8.0);

                    let install_dir = crate::installer::get_default_install_dir();
                    ui.label(RichText::new(format!("Default Location: {}", install_dir.display())).weak().size(11.0));
                    ui.add_space(8.0);

                    ui.checkbox(&mut self.install_opt_desktop, "Create Desktop shortcut");
                    ui.checkbox(&mut self.install_opt_start_menu, "Create Start Menu entry");
                    ui.checkbox(&mut self.install_opt_context_menu, "Add \"RenameAll\" to Windows Explorer right-click context menu");

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Apply / Install Setup").strong()).clicked() {
                            match crate::installer::install_to_default_location(
                                self.install_opt_desktop,
                                self.install_opt_start_menu,
                                self.install_opt_context_menu,
                            ) {
                                Ok(path) => {
                                    self.shell_registered = crate::installer::is_context_menu_installed();
                                    self.status_message = Some((
                                        format!("Successfully configured/installed to {}", path.display()),
                                        false,
                                    ));
                                    self.show_install_dialog = false;
                                }
                                Err(e) => {
                                    self.status_message = Some((format!("Setup failed: {}", e), true));
                                }
                            }
                        }

                        if ui.button("Remove Shortcuts & Menu").clicked() {
                            crate::installer::uninstall();
                            self.install_opt_desktop = false;
                            self.install_opt_start_menu = false;
                            self.install_opt_context_menu = false;
                            self.shell_registered = false;
                            self.status_message = Some(("Shortcuts and context menu removed.".into(), false));
                            self.show_install_dialog = false;
                        }

                        if ui.button("Close").clicked() {
                            self.show_install_dialog = false;
                        }
                    });
                    ui.add_space(4.0);
                });

            if !is_open {
                self.show_install_dialog = false;
            }
        }

        // Practical Recipe Guide Modal Dialog
        if self.show_recipes_dialog {
            crate::ui::recipes::render_recipe_window(
                ctx,
                &mut self.show_recipes_dialog,
                &mut self.config,
                &mut self.is_dirty,
                &mut self.status_message,
            );
        }
    }
}
