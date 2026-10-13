use std::path::PathBuf;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/RenameAll.ico");
        res.set("FileDescription", "Bulk Renamer");
        res.set("ProductName", "RenameAll");
        res.set("OriginalFilename", "RenameAll.exe");
        res.set("InternalName", "RenameAll");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile windows resource: {}", e);
        }

        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let res_o = out_dir.join("resource.o");
        if res_o.exists() {
            println!("cargo:rustc-link-arg={}", res_o.display());
        }
    }
}
