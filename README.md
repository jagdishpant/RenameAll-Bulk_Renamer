# RenameAll-Bulk_Renamer
A lightweight, smooth and blazing-fast batch file renaming utility for inspired by Microsoft PowerToys PowerRename, written in pure **Rust**.

## 🚀 Key Features

- **⚡ Blazing Fast & Lightweight**:
  - Compiles to a single native Windows executable with zero runtime dependencies (no Electron, no WebView2 bloat).
  - Immediate-mode GUI ('egui' + OpenGL) rendering at a silky-smooth 60+ FPS.
  - Near-instant startup (~15 ms) and minimal RAM usage (~20 MB).

- **🎨 Modern Visual Styling**:
  - Dark theme support
  - Windows rounded corners, subtle borders, and intuitive layout.

- **🔍 Search & Replace**:
  - Real-time instant preview as you type.
  - Regular Expressions (Regex) with full capture group substitution.
  - Plain-text matching with optional "Case Sensitivity".
  - Global replace (Match All Occurrences) or first match only.

- **🎯 Granular Targeting**:
  - **Filename + Extension**: Rename the full filename.
  - **Filename only**: Keeps original file extension intact.
  - **Extension only**: Batch convert extensions (e.g. '.jpeg' -> '.jpg').

- **🔤 Text Case Formatting**:
  - lowercase (e.g., 'document_sample.pdf')
  - UPPERCASE (e.g., 'DOCUMENT_SAMPLE.PDF')
  - Title Case (e.g., 'Document Sample.pdf')
  - Capitalize first word (e.g., Document sample.pdf)

- **🔢 Sequential Numbering & Dynamic Tokens**:
  - Suffix or Prefix sequential numbering with customizable start index, step, and zero padding ('_001', '_002', etc.).
  - Token substitution in replacement string:
    - '${count}' / '${padding}': Sequence counter
    - '${YYYY}' / '${YY}': Current year (4-digit / 2-digit)
    - '${MM}' / '${DD}': Current month / day
    - '${hh}' / '${mm}' / '${ss}': Current hour / minute / second

- **🛡️ Safety & Collision Prevention**:
  - Live conflict detection: warns if two items would collide or overwrite existing files.
  - Windows filename validation: flags forbidden characters ('< > : " / \ | ? *') and reserved device names ('CON', 'PRN', 'NUL', etc.).
  - Safe Windows NTFS case renames ('foo.txt' -> 'FOO.txt') via intermediate atomic temp files.
  - Reversible: Easily revert your last batch rename operation in reverse order.

- **🖱️ Windows Integration & Drag-and-Drop**:
  - Drag and drop files and folders directly from Windows Explorer into the application window.
  - Add native file/folder pickers.
  - Accepts command-line arguments: 'RenameAll.exe <files/folders...>'.

---

