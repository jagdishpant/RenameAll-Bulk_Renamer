use std::path::PathBuf;
use egui::{Align, Color32, Layout, RichText, ScrollArea, Stroke, Ui};

use crate::engine::{EnumeratePosition, RenameConfig, TargetScope};

/// Attempts to open the DOCUMENTATION.md file using Windows default viewer
pub fn open_documentation_file() {
    let mut possible_paths = Vec::new();

    // 1. Current working directory
    possible_paths.push(PathBuf::from("DOCUMENTATION.md"));

    // 2. Next to running executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            possible_paths.push(parent.join("DOCUMENTATION.md"));
            if let Some(grandparent) = parent.parent() {
                possible_paths.push(grandparent.join("DOCUMENTATION.md"));
            }
        }
    }

    // 3. Known project directory
    possible_paths.push(PathBuf::from(
        r"C:\Users\Bhumika Pant\Music\Project\FCC\RenameAll_Bulk-Renamer\DOCUMENTATION.md",
    ));

    for path in possible_paths {
        if path.exists() {
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", &path.to_string_lossy()])
                .spawn();
            return;
        }
    }
}

fn recipe_card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let bg_color = if ui.visuals().dark_mode {
        Color32::from_rgb(30, 33, 40)
    } else {
        Color32::from_rgb(246, 248, 252)
    };
    let border_color = if ui.visuals().dark_mode {
        Color32::from_rgb(52, 58, 70)
    } else {
        Color32::from_rgb(215, 222, 232)
    };

    egui::Frame::none()
        .fill(bg_color)
        .stroke(Stroke::new(1.0_f32, border_color))
        .rounding(6.0)
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, add_contents)
        .inner
}

pub fn render_recipe_window(
    ctx: &egui::Context,
    open: &mut bool,
    config: &mut RenameConfig,
    is_dirty: &mut bool,
    status_message: &mut Option<(String, bool)>,
) {
    let mut is_open = *open;
    let mut close_clicked = false;

    // Close on pressing Escape key
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close_clicked = true;
    }

    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 40.0).max(320.0);
    let max_h = (screen.height() - 40.0).max(240.0);
    let default_w = 780.0_f32.min(max_w);
    let default_h = 560.0_f32.min(max_h);

    egui::Window::new(" Practical Recipe Guide - Examples")
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .constrain(true)
        .max_width(max_w)
        .max_height(max_h)
        .default_size([default_w, default_h])
        .min_width(360.0)
        .min_height(240.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.add_space(2.0);
            ui.label(
                RichText::new("Examples & Recipes from DOCUMENTATION.md")
                    .strong()
                    .size(15.0),
            );
            ui.label(
                RichText::new(
                    "Click ' Apply Recipe' on any recipe below to instantly load its patterns and settings into the program. Press Esc to close.",
                )
                .weak()
                .size(12.0),
            );
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // Reserve height at the bottom for separator and action buttons so the Close button is always visible
            let available_for_scroll = (ui.available_height() - 44.0).max(80.0);

            ScrollArea::vertical()
                .id_salt("recipe_guide_scroll")
                .max_height(available_for_scroll)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // -------------------------------------------------------------
                    // Recipe 1: Add a Prefix to Filenames
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new("1. Add a Prefix to Filenames").size(14.0));
                        ui.label(RichText::new("Prepend a tag, date, or category to the start of files without altering extensions.\nExample Goal: report.pdf -> 2026_Q4_report.pdf").size(12.0));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Method A: Regular Expression").strong().size(12.0));
                                ui.label("- Search: ^ (caret indicates start of filename)");
                                ui.label("- Replace with: 2026_Q4_");
                                ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                                if ui.button(RichText::new(" Apply Regex Prefix Recipe").strong()).clicked() {
                                    config.search = "^".to_string();
                                    config.replace = "2026_Q4_".to_string();
                                    config.use_regex = true;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = false;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Prefix with Regex (^)".into(), false));
                                }
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.label(RichText::new("Method B: Sequential Counter").strong().size(12.0));
                                ui.label("- Enable Item Counter: Checked");
                                ui.label("- Position: Prefix | Separator: _ | Padding: 2");
                                ui.label("- Produces: 01_report.pdf, 02_invoice.pdf...");
                                if ui.button(RichText::new(" Apply Counter Prefix Recipe").strong()).clicked() {
                                    config.search.clear();
                                    config.replace.clear();
                                    config.use_regex = false;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = true;
                                    config.enumerate.position = EnumeratePosition::Prefix;
                                    config.enumerate.separator = "_".to_string();
                                    config.enumerate.padding = 2;
                                    config.enumerate.start = 1;
                                    config.enumerate.step = 1;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Prefix with Counter".into(), false));
                                }
                            });
                        });
                    });

                    ui.add_space(8.0);

                    // -------------------------------------------------------------
                    // Recipe 2: Add a Suffix to Filenames
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new("2. Add a Suffix to Filenames").size(14.0));
                        ui.label(RichText::new("Append a status, version, or label to the end of filenames before extension.\nExample Goal: document.docx -> document_FINAL.docx").size(12.0));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Method A: Regular Expression").strong().size(12.0));
                                ui.label("- Search: $ (dollar sign indicates end of filename)");
                                ui.label("- Replace with: _FINAL");
                                ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                                if ui.button(RichText::new(" Apply Regex Suffix Recipe").strong()).clicked() {
                                    config.search = "$".to_string();
                                    config.replace = "_FINAL".to_string();
                                    config.use_regex = true;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = false;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Suffix with Regex ($)".into(), false));
                                }
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.label(RichText::new("Method B: Sequential Counter").strong().size(12.0));
                                ui.label("- Enable Item Counter: Checked");
                                ui.label("- Position: Suffix | Separator: _ | Padding: 3");
                                ui.label("- Produces: document_001.docx, document_002.docx...");
                                if ui.button(RichText::new(" Apply Counter Suffix Recipe").strong()).clicked() {
                                    config.search.clear();
                                    config.replace.clear();
                                    config.use_regex = false;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = true;
                                    config.enumerate.position = EnumeratePosition::Suffix;
                                    config.enumerate.separator = "_".to_string();
                                    config.enumerate.padding = 3;
                                    config.enumerate.start = 1;
                                    config.enumerate.step = 1;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Suffix with Counter".into(), false));
                                }
                            });
                        });
                    });

                    ui.add_space(8.0);

                    // -------------------------------------------------------------
                    // Recipe 3: Replace All File Names with New Numbered Names
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new("3. Replace All File Names with New Numbered Names").size(14.0));
                        ui.label(RichText::new("Reset messy camera or scanner names into a clean, uniform numbered series.\nExample Goal: IMG_9041.JPG, DSC_0012.JPG -> Vacation_001.JPG, Vacation_002.JPG...").size(12.0));
                        ui.add_space(6.0);
                        ui.label("- Search: .* (matches entire existing filename)");
                        ui.label("- Replace with: Vacation_${count} (or Vacation_${YYYY}_${count})");
                        ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                        ui.label("- Item Counter: Start: 1, Step: 1, Padding: 3");
                        ui.add_space(4.0);
                        if ui.button(RichText::new(" Apply Numbered Names Recipe").strong()).clicked() {
                            config.search = ".*".to_string();
                            config.replace = "Vacation_${count}".to_string();
                            config.use_regex = true;
                            config.scope = TargetScope::FilenameOnly;
                            config.enumerate.enabled = true;
                            config.enumerate.position = EnumeratePosition::None;
                            config.enumerate.start = 1;
                            config.enumerate.step = 1;
                            config.enumerate.padding = 3;
                            *is_dirty = true;
                            *status_message = Some(("Loaded Recipe: Numbered Series (${count})".into(), false));
                        }
                    });

                    ui.add_space(8.0);

                    // -------------------------------------------------------------
                    // Recipe 4: Replace Filename Before a Particular Character
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new("4. Replace Filename Before a Particular Character").size(14.0));
                        ui.label(RichText::new("Target and update or strip metadata prefixes that precede a delimiter (like '-').").size(12.0));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Case 4A: Replace prefix before hyphen").strong().size(12.0));
                                ui.label("OLD_DEP-Budget2026.xlsx -> FINANCE-Budget2026.xlsx");
                                ui.label("- Search: ^[^-]*-");
                                ui.label("- Replace with: FINANCE-");
                                ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                                if ui.button(RichText::new(" Apply Case 4A Recipe").strong()).clicked() {
                                    config.search = "^[^-]*-".to_string();
                                    config.replace = "FINANCE-".to_string();
                                    config.use_regex = true;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = false;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Replace before hyphen".into(), false));
                                }
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.label(RichText::new("Case 4B: Strip prefix before hyphen").strong().size(12.0));
                                ui.label("OLD_DEP-Budget2026.xlsx -> Budget2026.xlsx");
                                ui.label("- Search: ^[^-]*-");
                                ui.label("- Replace with: (empty)");
                                ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                                if ui.button(RichText::new(" Apply Case 4B Recipe").strong()).clicked() {
                                    config.search = "^[^-]*-".to_string();
                                    config.replace.clear();
                                    config.use_regex = true;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = false;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Strip before hyphen".into(), false));
                                }
                            });
                        });
                    });

                    ui.add_space(8.0);

                    // -------------------------------------------------------------
                    // Recipe 5: Replace Filename After a Particular Character
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new("5. Replace Filename After a Particular Character").size(14.0));
                        ui.label(RichText::new("Target and update or truncate suffixes following a delimiter (like '-').").size(12.0));
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Case 5A: Replace suffix after hyphen").strong().size(12.0));
                                ui.label("Report-Draft_v1.pdf -> Report-Approved.pdf");
                                ui.label("- Search: -[^-]*$");
                                ui.label("- Replace with: -Approved");
                                ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                                if ui.button(RichText::new(" Apply Case 5A Recipe").strong()).clicked() {
                                    config.search = "-[^-]*$".to_string();
                                    config.replace = "-Approved".to_string();
                                    config.use_regex = true;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = false;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Replace after hyphen".into(), false));
                                }
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.label(RichText::new("Case 5B: Strip suffix after hyphen").strong().size(12.0));
                                ui.label("ProjectName-temp_copy.zip -> ProjectName.zip");
                                ui.label("- Search: -[^-]*$");
                                ui.label("- Replace with: (empty)");
                                ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                                if ui.button(RichText::new(" Apply Case 5B Recipe").strong()).clicked() {
                                    config.search = "-[^-]*$".to_string();
                                    config.replace.clear();
                                    config.use_regex = true;
                                    config.scope = TargetScope::FilenameOnly;
                                    config.enumerate.enabled = false;
                                    *is_dirty = true;
                                    *status_message = Some(("Loaded Recipe: Strip after hyphen".into(), false));
                                }
                            });
                        });
                    });

                    ui.add_space(8.0);

                    // -------------------------------------------------------------
                    // Recipe 6: Change Character Positions (Reorder Segments)
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new("6. Change Character Positions (Reorder Segments)").size(14.0));
                        ui.label(RichText::new("Rearrange structured filename components (such as division codes, employee IDs, and names) using capture groups.\nExample Goal: DIV-EMP-Name.txt -> EMP-DIV-Name.txt (e.g. HR-1042-JohnDoe.txt -> 1042-HR-JohnDoe.txt)").size(12.0));
                        ui.add_space(6.0);
                        ui.label("- Search: ^([^-]+)-([^-]+)-(.*)$");
                        ui.label("- Replace with: $2-$1-$3");
                        ui.label("- Regular Expressions: Checked | Apply To: Filename only");
                        ui.label(RichText::new("How it works: $1 matches first segment (HR), $2 matches second segment (1042), $3 matches remainder (JohnDoe). Reordering to $2-$1-$3 swaps the first two segments.").weak().size(11.0));
                        ui.add_space(4.0);
                        if ui.button(RichText::new(" Apply Segment Reorder Recipe").strong()).clicked() {
                            config.search = r"^([^-]+)-([^-]+)-(.*)$".to_string();
                            config.replace = "$2-$1-$3".to_string();
                            config.use_regex = true;
                            config.scope = TargetScope::FilenameOnly;
                            config.enumerate.enabled = false;
                            *is_dirty = true;
                            *status_message = Some(("Loaded Recipe: Reorder segments ($2-$1-$3)".into(), false));
                        }
                    });

                    ui.add_space(8.0);

                    // -------------------------------------------------------------
                    // Summary Reference Cheat Sheet
                    // -------------------------------------------------------------
                    recipe_card(ui, |ui| {
                        ui.heading(RichText::new(" Summary Reference Cheat Sheet").size(14.0));
                        ui.add_space(4.0);

                        egui::Grid::new("cheat_sheet_grid")
                            .striped(true)
                            .min_col_width(90.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Task").strong());
                                ui.label(RichText::new("Search").strong());
                                ui.label(RichText::new("Replace with").strong());
                                ui.label(RichText::new("Regex?").strong());
                                ui.label(RichText::new("Apply To").strong());
                                ui.end_row();

                                ui.label("1. Add Prefix");
                                ui.label("^");
                                ui.label("MyPrefix_");
                                ui.label("Yes");
                                ui.label("Filename only");
                                ui.end_row();

                                ui.label("2. Add Suffix");
                                ui.label("$");
                                ui.label("_MySuffix");
                                ui.label("Yes");
                                ui.label("Filename only");
                                ui.end_row();

                                ui.label("3. Clean Numbering");
                                ui.label(".*");
                                ui.label("Item_${count}");
                                ui.label("Yes");
                                ui.label("Filename only");
                                ui.end_row();

                                ui.label("4. Replace Before -");
                                ui.label("^[^-]*-");
                                ui.label("NewPrefix-");
                                ui.label("Yes");
                                ui.label("Filename only");
                                ui.end_row();

                                ui.label("5. Replace After -");
                                ui.label("-[^-]*$");
                                ui.label("-NewSuffix");
                                ui.label("Yes");
                                ui.label("Filename only");
                                ui.end_row();

                                ui.label("6. Swap Segments (A-B-C)");
                                ui.label("^([^-]+)-([^-]+)-(.*)$");
                                ui.label("$2-$1-$3");
                                ui.label("Yes");
                                ui.label("Filename only");
                                ui.end_row();

                                ui.label("Batch Extension Change");
                                ui.label(".*");
                                ui.label("jpg");
                                ui.label("Yes");
                                ui.label("Extension only");
                                ui.end_row();
                            });

                        ui.add_space(6.0);
                        if ui.button(RichText::new(" Apply Extension Change Recipe (.jpg)").strong()).clicked() {
                            config.search = ".*".to_string();
                            config.replace = "jpg".to_string();
                            config.use_regex = true;
                            config.scope = TargetScope::ExtensionOnly;
                            config.enumerate.enabled = false;
                            *is_dirty = true;
                            *status_message = Some(("Loaded Recipe: Batch Extension Change".into(), false));
                        }
                    });

                    ui.add_space(12.0);
                });

            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new(" Open Full DOCUMENTATION.md").size(12.0)).clicked() {
                    open_documentation_file();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(RichText::new("Close (Esc)").size(12.0)).clicked() {
                        close_clicked = true;
                    }
                });
            });
        });

    if close_clicked {
        is_open = false;
    }

    *open = is_open;
}
