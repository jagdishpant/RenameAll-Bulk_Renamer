use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetScope {
    FilenameAndExtension,
    FilenameOnly,
    ExtensionOnly,
}

impl TargetScope {
    pub fn label(&self) -> &'static str {
        match self {
            TargetScope::FilenameAndExtension => "Filename + Extension",
            TargetScope::FilenameOnly => "Filename only",
            TargetScope::ExtensionOnly => "Extension only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextFormat {
    None,
    Lowercase,
    Uppercase,
    TitleCase,
    CapitalizeFirst,
}

impl TextFormat {
    pub fn label(&self) -> &'static str {
        match self {
            TextFormat::None => "Original (No change)",
            TextFormat::Lowercase => "lowercase",
            TextFormat::Uppercase => "UPPERCASE",
            TextFormat::TitleCase => "Title Case",
            TextFormat::CapitalizeFirst => "Capitalize first word",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumeratePosition {
    None,
    Suffix,
    Prefix,
}

impl EnumeratePosition {
    pub fn label(&self) -> &'static str {
        match self {
            EnumeratePosition::None => "None (or use ${count} in replace)",
            EnumeratePosition::Suffix => "Suffix (e.g. name_01.ext)",
            EnumeratePosition::Prefix => "Prefix (e.g. 01_name.ext)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumerateConfig {
    pub enabled: bool,
    pub position: EnumeratePosition,
    pub start: u32,
    pub step: u32,
    pub padding: usize,
    pub separator: String,
}

impl Default for EnumerateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: EnumeratePosition::None,
            start: 1,
            step: 1,
            padding: 2,
            separator: "_".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameConfig {
    pub search: String,
    pub replace: String,
    pub use_regex: bool,
    pub match_all: bool,
    pub case_sensitive: bool,
    pub scope: TargetScope,
    pub format: TextFormat,
    pub enumerate: EnumerateConfig,
    pub include_files: bool,
    pub include_folders: bool,
    pub include_subfolders: bool,
}

impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            search: String::new(),
            replace: String::new(),
            use_regex: false,
            match_all: true,
            case_sensitive: false,
            scope: TargetScope::FilenameOnly,
            format: TextFormat::None,
            enumerate: EnumerateConfig::default(),
            include_files: true,
            include_folders: true,
            include_subfolders: false,
        }
    }
}
