# ⚡ RenameAll - Bulk Renamer

A lightweight, smooth, and blazing-fast batch file renaming utility for Windows inspired by **Microsoft PowerToys PowerRename**, written in pure **Rust**.

---

## 📦 Windows Installer

A standard Windows installer wizard is generated at:
```text
dist\RenameAll_Setup.exe
```

### Installer Capabilities:
1. **Default Installation Directory**:
   - `C:\Program Files\RenameAll` (when run with administrator privileges)
   - `%LOCALAPPDATA%\Programs\RenameAll` (when run in per-user mode without admin prompt)
2. **Setup Tasks / Checkboxes**:
   - ☑ **Create a Desktop shortcut** (`RenameAll.lnk`)
   - ☑ **Create a Start Menu shortcut**
   - ☑ **Add "RenameAll" to Windows Explorer right-click context menu**
3. **Clean Windows Uninstaller**:
   - Registered under Windows Settings > Apps > **Installed apps** (**Apps & features**).
   - Uninstalls cleanly, removing program files, shortcuts, and registry context menu entries.

---

## 🎨 Branding & Icons
- **Program Title**: `RenameAll`
- **Description**: `Bulk Renamer`
- **Executable**: `RenameAll.exe`
- **Installer**: `RenameAll_Setup.exe`
- **Icon Integration**:
  - Embedded Windows PE Resource (`RenameAll.ico` embedded directly into the `.exe` binary).
  - Native Windows taskbar and title bar icon (`RenameAll.png` RGBA texture).
  - Modern in-app branding header logo and empty-state dropzone branding.
  - Explorer Right-Click Context Menu with embedded icon.

---

## 🛠️ Project Structure

```text
power_rename/
├── Cargo.toml                # Project metadata & optimized dependencies
├── build.rs                  # Windows resource script embedding RenameAll.ico & metadata
├── build_installer.bat       # 1-Click script to compile release binary and Inno Setup installer
├── assets/
│   ├── RenameAll.ico         # Multi-resolution Windows icon (.ico)
│   └── RenameAll.png         # High-resolution PNG logo
├── dist/
│   └── RenameAll_Setup.exe   # Standard Windows installer wizard (8.1 MB)
├── installer/
│   └── RenameAll.iss         # Inno Setup 6 compilation script
├── src/
│   ├── main.rs              # Native entry point (CLI args & hidden console subsystem)
│   ├── lib.rs               # Library root
│   ├── installer.rs         # Programmatic shortcut & context menu management
│   ├── engine/              # Pure Rust renaming logic, regex, & safety validation
│   └── ui/                  # Fast GUI powered by egui / eframe
└── tests/
    ├── doc_examples_tests.rs # Automated test suite for recipe examples
    ├── engine_tests.rs      # Unit test suite for rules, regex, tokens & collisions
    └── fs_tests.rs          # Real filesystem transactional rename & undo tests
```

---

## 💻 How to Build & Run

### 1. Launch the Standard Windows Installer
```powershell
& "C:\Users\ITO\.gemini\antigravity\scratch\power_rename\dist\RenameAll_Setup.exe"
```

### 2. Rebuild the Installer
Run the batch script:
```cmd
build_installer.bat
```
Or manually with `iscc`:
```powershell
& "C:\Users\ITO\AppData\Local\Programs\Inno Setup 6\ISCC.exe" "installer\RenameAll.iss"
```

### 3. Run Standalone Binary Directly
```powershell
& "C:\Users\ITO\.gemini\antigravity\scratch\power_rename\target\release\RenameAll.exe"
```
Or via CLI with self-installer flags:
```powershell
# Self-installs to %LOCALAPPDATA%\Programs\RenameAll with shortcuts and context menu
.\target\release\RenameAll.exe --install

# Cleans up shortcuts and context menu
.\target\release\RenameAll.exe --uninstall
```
