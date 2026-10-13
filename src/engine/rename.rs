use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use regex::Regex;
use chrono::Local;

use super::rules::{EnumeratePosition, RenameConfig, TargetScope, TextFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStatus {
    Unchanged,
    Ready,
    Conflict(String),
    Invalid(String),
}

impl ItemStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, ItemStatus::Ready)
    }

    #[allow(dead_code)]
    pub fn badge_text(&self) -> &'static str {
        match self {
            ItemStatus::Unchanged => "Unchanged",
            ItemStatus::Ready => "Will Rename",
            ItemStatus::Conflict(_) => "Conflict",
            ItemStatus::Invalid(_) => "Invalid",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileItem {
    #[allow(dead_code)]
    pub id: usize,
    pub original_path: PathBuf,
    pub directory: PathBuf,
    pub original_name: String,
    pub new_name: String,
    pub is_directory: bool,
    pub selected: bool,
    pub status: ItemStatus,
}

#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub current_path: PathBuf,
    pub previous_path: PathBuf,
}

/// Validates Windows filename constraints
pub fn validate_windows_filename(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Filename cannot be empty");
    }
    if name.len() > 255 {
        return Err("Filename exceeds 255 characters");
    }
    // Check forbidden characters
    let forbidden = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if name.chars().any(|c| forbidden.contains(&c) || (c as u32) < 32) {
        return Err("Contains illegal Windows characters (< > : \" / \\ | ? *)");
    }
    // Trailing space or dot is invalid in Windows
    if name.ends_with(' ') || name.ends_with('.') {
        return Err("Cannot end with a period or space");
    }
    // Check reserved Windows device names
    let stem = if let Some(idx) = name.find('.') {
        &name[..idx]
    } else {
        name
    };
    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem_upper = stem.to_uppercase();
    if reserved.contains(&stem_upper.as_str()) {
        return Err("Reserved Windows device name");
    }

    Ok(())
}

/// Transforms text according to the selected case convention
pub fn transform_text(text: &str, format: TextFormat) -> String {
    match format {
        TextFormat::None => text.to_string(),
        TextFormat::Lowercase => text.to_lowercase(),
        TextFormat::Uppercase => text.to_uppercase(),
        TextFormat::TitleCase => {
            let mut result = String::with_capacity(text.len());
            let mut capitalize_next = true;
            for c in text.chars() {
                if c.is_alphabetic() {
                    if capitalize_next {
                        result.extend(c.to_uppercase());
                        capitalize_next = false;
                    } else {
                        result.extend(c.to_lowercase());
                    }
                } else {
                    result.push(c);
                    capitalize_next = !c.is_alphanumeric();
                }
            }
            result
        }
        TextFormat::CapitalizeFirst => {
            let mut chars = text.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}

/// Replaces date/time and sequential counter tokens
pub fn replace_tokens(pattern: &str, counter: u32, padding: usize) -> String {
    let now = Local::now();
    let formatted_count = format!("{:0width$}", counter, width = padding);

    let res = pattern
        .replace("${count}", &formatted_count)
        .replace("${padding}", &formatted_count)
        .replace("${YYYY}", &now.format("%Y").to_string())
        .replace("${YY}", &now.format("%y").to_string())
        .replace("${MM}", &now.format("%m").to_string())
        .replace("${DD}", &now.format("%d").to_string())
        .replace("${hh}", &now.format("%H").to_string())
        .replace("${mm}", &now.format("%M").to_string())
        .replace("${ss}", &now.format("%S").to_string());

    res
}

/// Computes the new name for an individual item
pub fn compute_name(
    original_name: &str,
    is_directory: bool,
    config: &RenameConfig,
    regex_obj: Option<&Regex>,
    item_counter: u32,
) -> Result<String, String> {
    // Determine split between stem and extension
    let (stem, ext, has_ext) = if !is_directory {
        if let Some(pos) = original_name.rfind('.') {
            // Edge case: hidden file starting with dot and no other dot (e.g. .gitignore)
            if pos == 0 {
                (original_name, "", false)
            } else {
                (&original_name[..pos], &original_name[pos + 1..], true)
            }
        } else {
            (original_name, "", false)
        }
    } else {
        (original_name, "", false)
    };

    let target_scope = if is_directory {
        TargetScope::FilenameAndExtension
    } else {
        config.scope
    };

    let (mut target_part, prefix_part, suffix_part) = match target_scope {
        TargetScope::FilenameAndExtension => (original_name.to_string(), String::new(), String::new()),
        TargetScope::FilenameOnly => {
            if has_ext {
                (stem.to_string(), String::new(), format!(".{}", ext))
            } else {
                (stem.to_string(), String::new(), String::new())
            }
        }
        TargetScope::ExtensionOnly => {
            if has_ext {
                (ext.to_string(), format!("{}.", stem), String::new())
            } else {
                // No extension to target
                return Ok(original_name.to_string());
            }
        }
    };

    // Apply search & replace
    if !config.search.is_empty() {
        let replace_with = replace_tokens(
            &config.replace,
            item_counter,
            config.enumerate.padding,
        );

        if config.use_regex {
            if let Some(re) = regex_obj {
                if config.match_all {
                    target_part = re.replace_all(&target_part, replace_with.as_str()).to_string();
                } else {
                    target_part = re.replace(&target_part, replace_with.as_str()).to_string();
                }
            }
        } else {
            // Plain text replacement
            if config.case_sensitive {
                if config.match_all {
                    target_part = target_part.replace(&config.search, &replace_with);
                } else if let Some(idx) = target_part.find(&config.search) {
                    target_part.replace_range(idx..idx + config.search.len(), &replace_with);
                }
            } else {
                // Case-insensitive plain text replace
                let lower_target = target_part.to_lowercase();
                let lower_search = config.search.to_lowercase();
                if config.match_all {
                    let mut result = String::new();
                    let mut last_idx = 0;
                    for (idx, _) in lower_target.match_indices(&lower_search) {
                        result.push_str(&target_part[last_idx..idx]);
                        result.push_str(&replace_with);
                        last_idx = idx + lower_search.len();
                    }
                    result.push_str(&target_part[last_idx..]);
                    target_part = result;
                } else if let Some(idx) = lower_target.find(&lower_search) {
                    target_part.replace_range(idx..idx + lower_search.len(), &replace_with);
                }
            }
        }
    }

    // Apply text format (lowercase, UPPERCASE, etc.)
    target_part = transform_text(&target_part, config.format);

    // Apply enumeration if enabled
    if config.enumerate.enabled {
        let formatted_count = format!(
            "{:0width$}",
            item_counter,
            width = config.enumerate.padding
        );
        match config.enumerate.position {
            EnumeratePosition::Prefix => {
                target_part = format!(
                    "{}{}{}",
                    formatted_count, config.enumerate.separator, target_part
                );
            }
            EnumeratePosition::Suffix => {
                target_part = format!(
                    "{}{}{}",
                    target_part, config.enumerate.separator, formatted_count
                );
            }
            EnumeratePosition::None => {}
        }
    }

    let final_name = format!("{}{}{}", prefix_part, target_part, suffix_part);
    Ok(final_name)
}

/// Updates previews and recalculates collisions & statuses for all loaded items
pub fn update_preview_list(items: &mut [FileItem], config: &RenameConfig) -> Result<(), String> {
    // Compile regex if enabled
    let regex_compiled = if config.use_regex && !config.search.is_empty() {
        let pattern = if config.case_sensitive {
            config.search.clone()
        } else {
            format!("(?i){}", config.search)
        };
        match Regex::new(&pattern) {
            Ok(re) => Some(re),
            Err(e) => return Err(format!("Regex error: {}", e)),
        }
    } else {
        None
    };

    // Calculate sequential counters for selected items
    let mut counter = config.enumerate.start;

    for item in items.iter_mut() {
        let current_count = counter;
        if item.selected {
            counter = counter.saturating_add(config.enumerate.step);
        }

        match compute_name(
            &item.original_name,
            item.is_directory,
            config,
            regex_compiled.as_ref(),
            current_count,
        ) {
            Ok(name) => {
                item.new_name = name;
            }
            Err(e) => {
                item.new_name = item.original_name.clone();
                item.status = ItemStatus::Invalid(e);
                continue;
            }
        }

        // Validate filename syntax
        if let Err(err_msg) = validate_windows_filename(&item.new_name) {
            item.status = ItemStatus::Invalid(err_msg.to_string());
        } else if item.new_name == item.original_name {
            item.status = ItemStatus::Unchanged;
        } else {
            item.status = ItemStatus::Ready;
        }
    }

    // Detect collision between items targeting the same directory
    // Key: (directory_path, lowercase_new_name) -> list of item indices
    let mut target_map: HashMap<(PathBuf, String), Vec<usize>> = HashMap::new();
    for (idx, item) in items.iter().enumerate() {
        if item.selected && item.status == ItemStatus::Ready {
            let key = (item.directory.clone(), item.new_name.to_lowercase());
            target_map.entry(key).or_default().push(idx);
        }
    }

    for ((dir, _name_lower), indices) in target_map {
        if indices.len() > 1 {
            for &idx in &indices {
                items[idx].status = ItemStatus::Conflict(format!(
                    "Duplicate name collision with {} other item(s) in same folder",
                    indices.len() - 1
                ));
            }
        } else if let Some(&idx) = indices.first() {
            // Check if destination exists on disk and is NOT one of the renaming items
            let dest_path = dir.join(&items[idx].new_name);
            if dest_path.exists() {
                // If it exists, check whether it is the same file (e.g. only case change on Windows)
                // or if it belongs to another selected file in our list
                let is_self = dest_path == items[idx].original_path;
                let is_other_in_batch = items.iter().any(|other| other.original_path == dest_path);

                if !is_self && !is_other_in_batch {
                    items[idx].status = ItemStatus::Conflict(
                        "A file or folder with this name already exists on disk".to_string(),
                    );
                }
            }
        }
    }

    Ok(())
}

/// Executes safe transactional renaming
pub fn execute_batch_rename(items: &mut [FileItem]) -> (Vec<UndoEntry>, Vec<String>) {
    let mut undo_history: Vec<UndoEntry> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Filter to selected items that are Ready
    let ready_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.selected && item.status.is_ready())
        .map(|(idx, _)| idx)
        .collect();

    for idx in ready_indices {
        let item = &items[idx];
        let original_path = &item.original_path;
        let new_path = item.directory.join(&item.new_name);

        if original_path == &new_path {
            continue;
        }

        // Handle Windows NTFS case-insensitivity:
        // If renaming 'file.txt' to 'FILE.txt', standard rename might fail or be a no-op.
        // We use an intermediate temporary rename step.
        let is_case_only_rename = original_path
            .to_string_lossy()
            .eq_ignore_ascii_case(&new_path.to_string_lossy());

        let result = if is_case_only_rename {
            let temp_name = format!(
                "__rename_tmp_{}_{}.tmp",
                std::process::id(),
                chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
            );
            let temp_path = item.directory.join(temp_name);
            match fs::rename(original_path, &temp_path) {
                Ok(_) => match fs::rename(&temp_path, &new_path) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        // Attempt rollback of temp
                        let _ = fs::rename(&temp_path, original_path);
                        Err(e)
                    }
                },
                Err(e) => Err(e),
            }
        } else {
            fs::rename(original_path, &new_path)
        };

        match result {
            Ok(_) => {
                undo_history.push(UndoEntry {
                    current_path: new_path.clone(),
                    previous_path: original_path.clone(),
                });
                // Update item state
                let item_mut = &mut items[idx];
                item_mut.original_path = new_path;
                item_mut.original_name = item_mut.new_name.clone();
                item_mut.status = ItemStatus::Unchanged;
            }
            Err(e) => {
                errors.push(format!(
                    "Failed to rename '{}' -> '{}': {}",
                    item.original_name, item.new_name, e
                ));
            }
        }
    }

    (undo_history, errors)
}

/// Executes undo of a batch rename in reverse order
pub fn execute_undo(undo_history: Vec<UndoEntry>) -> (usize, Vec<String>) {
    let mut reverted_count = 0;
    let mut errors = Vec::new();

    // Revert in reverse order
    for entry in undo_history.into_iter().rev() {
        if !entry.current_path.exists() {
            errors.push(format!(
                "Cannot revert '{}': path does not exist",
                entry.current_path.display()
            ));
            continue;
        }

        let is_case_only = entry
            .current_path
            .to_string_lossy()
            .eq_ignore_ascii_case(&entry.previous_path.to_string_lossy());

        let result = if is_case_only {
            let temp_path = entry.current_path.with_extension(format!(
                "undo_tmp_{}",
                chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
            ));
            match fs::rename(&entry.current_path, &temp_path) {
                Ok(_) => match fs::rename(&temp_path, &entry.previous_path) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        let _ = fs::rename(&temp_path, &entry.current_path);
                        Err(e)
                    }
                },
                Err(e) => Err(e),
            }
        } else {
            fs::rename(&entry.current_path, &entry.previous_path)
        };

        match result {
            Ok(_) => reverted_count += 1,
            Err(e) => errors.push(format!(
                "Failed to revert '{}' -> '{}': {}",
                entry.current_path.display(),
                entry.previous_path.display(),
                e
            )),
        }
    }

    (reverted_count, errors)
}
