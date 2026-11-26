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
    pub is_subfolder: bool,
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

/// A case-only rename is not a collision with itself. Inspect directory entries
/// using their actual names instead of canonicalized paths: on Windows,
/// canonicalize() may preserve the requested spelling and cannot reliably
/// distinguish the source from its differently-cased destination.
///
/// Refuse the rename if a *different* sibling has the desired name, including
/// case-sensitive Windows directories that can contain case variants.
fn case_only_target_is_self(item: &FileItem) -> std::io::Result<bool> {
    if item.original_name == item.new_name
        || item.original_name.to_lowercase() != item.new_name.to_lowercase()
    {
        return Ok(false);
    }

    let mut source_found = false;
    for entry in fs::read_dir(&item.directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == item.original_name {
            source_found = true;
        } else if name.to_lowercase() == item.new_name.to_lowercase() {
            // A second, separately named directory entry would be overwritten
            // on case-sensitive directories, so never treat it as self.
            return Ok(false);
        }
    }
    Ok(source_found)
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
            // Treat a capitalization-only change as a rename of this same
            // directory entry, not a collision with its own Windows alias.
            // Inspect the *actual* directory entries rather than canonicalizing
            // two differently spelled paths. The latter gave false conflicts.
            let dest_path = dir.join(&items[idx].new_name);
            let is_self = case_only_target_is_self(&items[idx]).unwrap_or(false);
            if !is_self && dest_path.exists() {
                items[idx].status = ItemStatus::Conflict(
                    "A file or folder with this name already exists on disk".to_string(),
                );
            } else if !is_self && items[idx].original_name.to_lowercase()
                == items[idx].new_name.to_lowercase()
            {
                // A distinct case-variant may exist on a case-sensitive
                // directory even if `dest_path.exists()` is false.
                items[idx].status = ItemStatus::Conflict(
                    "Cannot safely identify the original file or folder for this case-only rename".to_string(),
                );
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


#[cfg(test)]
mod case_only_regression_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir() -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("renameall_case_{}_{}", std::process::id(), stamp));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn item(dir: &std::path::Path, name: &str) -> FileItem {
        FileItem {
            id: 0,
            original_path: dir.join(name),
            directory: dir.to_path_buf(),
            original_name: name.to_owned(),
            new_name: String::new(),
            is_directory: false,
            is_subfolder: false,
            selected: true,
            status: ItemStatus::Unchanged,
        }
    }

    #[test]
    fn different_existing_file_remains_conflict() {
        let dir = test_dir();
        fs::write(dir.join("one.txt"), b"one").unwrap();
        fs::write(dir.join("two.txt"), b"two").unwrap();
        let mut items = vec![item(&dir, "one.txt")];
        let mut config = RenameConfig::default();
        config.search = "one".into();
        config.replace = "two".into();
        update_preview_list(&mut items, &config).unwrap();
        assert!(matches!(items[0].status, ItemStatus::Conflict(_)));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn uppercase_case_preview_is_ready() {
        let dir = test_dir();
        fs::write(dir.join("report.txt"), b"example").unwrap();
        let mut items = vec![item(&dir, "report.txt")];
        let mut config = RenameConfig::default();
        config.format = TextFormat::Uppercase;
        update_preview_list(&mut items, &config).unwrap();
        assert_eq!(items[0].new_name, "REPORT.txt");
        assert!(matches!(items[0].status, ItemStatus::Ready), "{:?}", items[0].status);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lowercase_case_preview_is_ready() {
        let dir = test_dir();
        fs::write(dir.join("REPORT.txt"), b"example").unwrap();
        let mut items = vec![item(&dir, "REPORT.txt")];
        let mut config = RenameConfig::default();
        config.format = TextFormat::Lowercase;
        update_preview_list(&mut items, &config).unwrap();
        assert_eq!(items[0].new_name, "report.txt");
        assert!(matches!(items[0].status, ItemStatus::Ready), "{:?}", items[0].status);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn folder_case_preview_is_ready() {
        let dir = test_dir();
        fs::create_dir(dir.join("My Folder")).unwrap();
        let mut folder = item(&dir, "My Folder");
        folder.is_directory = true;
        let mut items = vec![folder];
        let mut config = RenameConfig::default();
        config.format = TextFormat::Lowercase;
        update_preview_list(&mut items, &config).unwrap();
        assert_eq!(items[0].new_name, "my folder");
        assert!(matches!(items[0].status, ItemStatus::Ready), "{:?}", items[0].status);
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn uppercase_only_rename_is_allowed_and_executed() {
        let dir = test_dir();
        fs::write(dir.join("report.txt"), b"example").unwrap();
        let mut items = vec![item(&dir, "report.txt")];
        let mut config = RenameConfig::default();
        config.scope = super::super::rules::TargetScope::FilenameAndExtension;
        config.format = TextFormat::Uppercase;
        update_preview_list(&mut items, &config).unwrap();
        assert_eq!(items[0].new_name, "REPORT.TXT");
        assert!(matches!(items[0].status, ItemStatus::Ready));
        let (_undo, errors) = execute_batch_rename(&mut items);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(items[0].original_name, "REPORT.TXT");
        assert!(dir.join("REPORT.TXT").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
