//! Embeds the application icon and version details in `rar-n-rush.exe`, so
//! that File Explorer, a pinned taskbar button, Alt+Tab and Task Manager show
//! the right icon and name.
//!
//! The icon is the one the program draws (`src/icon/art.rs`, included as is),
//! so there is no image file to keep in sync. The resource script is compiled
//! by the Windows SDK resource compiler (`rc.exe`), found by `embed-resource`.

#[path = "src/icon/art.rs"]
#[allow(dead_code)] // only the icon file is needed here
mod art;

use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=src/icon/art.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let ico = out.join("rar-n-rush.ico");
    fs::write(&ico, art::ico(&art::ICO_SIZES))?;

    let version = env::var("CARGO_PKG_VERSION")?;
    let numbers = ["MAJOR", "MINOR", "PATCH"]
        .map(|part| env::var(format!("CARGO_PKG_VERSION_{part}")).unwrap_or_else(|_| "0".into()))
        .join(",");
    // Backslashes are escapes in resource scripts.
    let ico_path = ico.display().to_string().replace('\\', "\\\\");
    let script = format!(
        r#"1 ICON "{ico_path}"

1 VERSIONINFO
FILEVERSION {numbers},0
PRODUCTVERSION {numbers},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Rar N Rush contributors"
      VALUE "FileDescription", "Rar N Rush"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "rar-n-rush"
      VALUE "LegalCopyright", "MIT OR Apache-2.0"
      VALUE "OriginalFilename", "rar-n-rush.exe"
      VALUE "ProductName", "Rar N Rush"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );
    let rc = out.join("rar-n-rush.rc");
    fs::write(&rc, script)?;
    embed_resource::compile(&rc, embed_resource::NONE).manifest_optional()?;
    Ok(())
}
