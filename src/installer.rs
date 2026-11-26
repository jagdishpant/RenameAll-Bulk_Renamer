use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the standard per-user default program install location:
/// %LOCALAPPDATA%\Programs\RenameAll
pub fn get_default_install_dir() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("Programs").join("RenameAll")
    } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
        PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("Programs")
            .join("RenameAll")
    } else {
        PathBuf::from("C:\\Program Files\\RenameAll")
    }
}

/// Returns the user's Desktop directory
pub fn get_desktop_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        PathBuf::from(profile).join("Desktop")
    } else {
        PathBuf::from("C:\\Users\\Public\\Desktop")
    }
}

/// Returns the user's Start Menu Programs directory
pub fn get_start_menu_programs_dir() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
    } else {
        PathBuf::from("C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs")
    }
}

pub fn get_desktop_shortcut_path() -> PathBuf {
    get_desktop_dir().join("RenameAll.lnk")
}

pub fn get_start_menu_shortcut_path() -> PathBuf {
    get_start_menu_programs_dir().join("RenameAll.lnk")
}

pub fn is_desktop_shortcut_installed() -> bool {
    get_desktop_shortcut_path().exists()
}

pub fn is_start_menu_shortcut_installed() -> bool {
    get_start_menu_shortcut_path().exists()
}

pub fn is_context_menu_installed() -> bool {
    // Query the registry in-process. Spawning reg.exe during GUI startup
    // can briefly flash a console window on Windows.
    use std::ptr::null_mut;
    use winapi::shared::minwindef::HKEY;
    use winapi::um::winreg::{RegCloseKey, RegOpenKeyExW, HKEY_CURRENT_USER};
    use winapi::um::winnt::KEY_READ;

    let key: Vec<u16> = "Software\\Classes\\*\\shell\\RenameAll"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut handle: HKEY = null_mut();
    let result = unsafe {
        RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_READ, &mut handle)
    };
    if result == 0 {
        unsafe { RegCloseKey(handle); }
        true
    } else {
        false
    }
}

/// Creates a Windows shell shortcut (.lnk) using WScript.Shell
pub fn create_shortcut(shortcut_path: &Path, target_exe: &Path) -> Result<(), String> {
    if let Some(parent) = shortcut_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{}'); \
         $s.TargetPath = '{}'; \
         $s.Description = 'Bulk Renamer'; \
         $s.IconLocation = '{},0'; \
         $s.Save()",
        shortcut_path.display(),
        target_exe.display(),
        target_exe.display()
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .output();

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).to_string()),
        Err(e) => Err(format!("Failed to execute powershell: {}", e)),
    }
}

pub fn remove_shortcut(shortcut_path: &Path) {
    if shortcut_path.exists() {
        let _ = fs::remove_file(shortcut_path);
    }
}

pub fn register_context_menu(exe_path: &Path) -> Result<(), String> {
    let exe_str = exe_path.to_string_lossy();
    let cmd = format!("\"{}\" \"%1\"", exe_str);
    let icon_spec = format!("\"{}\",0", exe_str);

    // Files
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/ve", "/d", "RenameAll", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/v", "Icon", "/d", &icon_spec, "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/v", "MultiSelectModel", "/d", "Player", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\*\\shell\\RenameAll\\command", "/ve", "/d", &cmd, "/f"])
        .output();

    // Folders
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/ve", "/d", "RenameAll", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/v", "Icon", "/d", &icon_spec, "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/v", "MultiSelectModel", "/d", "Player", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll\\command", "/ve", "/d", &cmd, "/f"])
        .output();

    // Folder Backgrounds
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll", "/ve", "/d", "RenameAll", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll", "/v", "Icon", "/d", &icon_spec, "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["add", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll\\command", "/ve", "/d", &cmd, "/f"])
        .output();

    Ok(())
}

pub fn unregister_context_menu() {
    let _ = Command::new("reg")
        .args(["delete", "HKCU\\Software\\Classes\\*\\shell\\RenameAll", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["delete", "HKCU\\Software\\Classes\\Directory\\shell\\RenameAll", "/f"])
        .output();
    let _ = Command::new("reg")
        .args(["delete", "HKCU\\Software\\Classes\\Directory\\Background\\shell\\RenameAll", "/f"])
        .output();
}

/// Installs the running executable to the default location and sets up requested options
pub fn install_to_default_location(
    create_desktop: bool,
    create_start_menu: bool,
    add_context_menu: bool,
) -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe().map_err(|e| format!("Cannot locate current exe: {}", e))?;
    let target_dir = get_default_install_dir();
    fs::create_dir_all(&target_dir).map_err(|e| format!("Cannot create target directory: {}", e))?;

    let target_exe = target_dir.join("RenameAll.exe");

    // Copy executable if not already running from target
    if current_exe != target_exe {
        fs::copy(&current_exe, &target_exe).map_err(|e| format!("Cannot copy executable to {}: {}", target_exe.display(), e))?;
    }

    if create_desktop {
        create_shortcut(&get_desktop_shortcut_path(), &target_exe)?;
    }

    if create_start_menu {
        create_shortcut(&get_start_menu_shortcut_path(), &target_exe)?;
    }

    if add_context_menu {
        register_context_menu(&target_exe)?;
    }

    Ok(target_exe)
}

/// Uninstalls shortcuts and context menu
pub fn uninstall() {
    remove_shortcut(&get_desktop_shortcut_path());
    remove_shortcut(&get_start_menu_shortcut_path());
    unregister_context_menu();
}
