// Ressources Windows des exécutables : icône (n° 1) et informations de version.
// Inclus par le build.rs de chaque crate qui produit un binaire : `include!(".../resources.rs");`
// puis `embed(description)`. Compilé avec windres (MinGW, dans le PATH du build).

fn embed(description: &str) {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets = manifest.join("..").join("..").join("assets");
    let icon = assets.join("fps-overlay.ico");
    println!("cargo:rerun-if-changed={}", icon.display());

    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();
    parts.resize(4, 0);
    let numeric = parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
    let icon_path = icon.display().to_string().replace('\\', "/");

    // UTF-8 : windres lit la page de code 65001 déclarée en tête
    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon_path}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040C04B0"
    BEGIN
      VALUE "CompanyName", "Rayane"
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{version}"
      VALUE "ProductName", "FPS Overlay"
      VALUE "ProductVersion", "{version}"
      VALUE "LegalCopyright", "MIT"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x040C, 1200
  END
END
"#
    );
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc_path = out.join("app.rc");
    let obj = out.join("app.res.o");
    std::fs::write(&rc_path, rc).unwrap();

    let status = std::process::Command::new("windres")
        .arg("--input-format=rc")
        .arg("--output-format=coff")
        .arg("-c")
        .arg("65001")
        .arg("-i")
        .arg(&rc_path)
        .arg("-o")
        .arg(&obj)
        .status();
    match status {
        Ok(s) if s.success() => println!("cargo:rustc-link-arg-bins={}", obj.display()),
        // sans windres le binaire reste fonctionnel, juste sans icône : on prévient sans échouer
        other => println!("cargo:warning=ressources Windows non intégrées (windres : {other:?})"),
    }
}
