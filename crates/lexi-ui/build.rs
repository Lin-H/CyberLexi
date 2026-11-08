//! Embed the CyberLexi icon + version resource into the Windows executable so
//! Explorer / taskbar / Alt-Tab show the brand mark. No-op on non-Windows
//! targets; a missing rc.exe is non-fatal (the icon is cosmetic), but a real
//! resource compile error still fails the build.
//!
//! The .rc is generated into OUT_DIR with an absolute icon path so rc.exe
//! resolves it regardless of its working directory. Version defaults to the
//! crate version; CI overrides it with the release tag via CYBERLEXI_VERSION.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico = manifest.join("../../packaging/icon/cyberlexi.ico");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", ico.display());
    println!("cargo:rerun-if-env-changed=CYBERLEXI_VERSION");

    let ver = std::env::var("CYBERLEXI_VERSION")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into()));
    let parts: Vec<&str> = ver.split('.').collect();
    let num = |i: usize| parts.get(i).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let (maj, min, pat) = (num(0), num(1), num(2));
    let ico_rc = ico.to_string_lossy().replace('\\', "\\\\");

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc = out.join("cyberlexi.rc");
    let src = format!(
        r#"1 ICON "{ico}"
1 VERSIONINFO
FILEVERSION {maj},{min},{pat},0
PRODUCTVERSION {maj},{min},{pat},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "CyberLexi"
            VALUE "FileDescription", "CyberLexi - Cyber English Dictionary"
            VALUE "FileVersion", "{ver}"
            VALUE "InternalName", "cyberlexi"
            VALUE "LegalCopyright", "MIT License"
            VALUE "OriginalFilename", "cyberlexi.exe"
            VALUE "ProductName", "CyberLexi"
            VALUE "ProductVersion", "{ver}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#,
        ico = ico_rc,
        maj = maj,
        min = min,
        pat = pat,
        ver = ver,
    );
    std::fs::write(&rc, src).expect("write generated .rc");

    embed_resource::compile(&rc, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}