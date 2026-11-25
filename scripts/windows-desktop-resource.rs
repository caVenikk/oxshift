// Embed the icon in the executable so Explorer and the native GPUI window
// resolve the same artwork. GPUI requests RT_GROUP_ICON number 1.
use std::{env, fmt::Write as _, fs, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let binary = env::var("CARGO_PKG_NAME").unwrap();
    let (name, icon) = match binary.as_str() {
        "oxshift-platform" => ("Oxshift", "oxshift.ico"),
        "oxshift-python" => ("Oxshift Python", "oxshift-python.ico"),
        _ => panic!("unknown desktop product {binary}"),
    };
    let artwork = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../assets/brand")
        .join(icon);
    println!("cargo:rerun-if-changed={}", artwork.display());
    let path = artwork
        .to_str()
        .expect("icon path must be Unicode")
        .replace('\\', "/")
        .replace('"', "\\\"");
    let major = env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
    let minor = env::var("CARGO_PKG_VERSION_MINOR").unwrap();
    let patch = env::var("CARGO_PKG_VERSION_PATCH").unwrap();
    let release = if patch == "0" {
        format!("{major}.{minor}")
    } else {
        format!("{major}.{minor}.{patch}")
    };
    let mut rc = format!("1 ICON \"{path}\"\n");
    writeln!(
        rc,
        r#"1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 1
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904B0"
  BEGIN
   VALUE "FileDescription", "{name}\0"
   VALUE "ProductName", "{name}\0"
   VALUE "FileVersion", "{release}\0"
   VALUE "ProductVersion", "{release}\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END"#
    )
    .unwrap();
    let resource = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("desktop.rc");
    fs::write(&resource, rc).expect("write desktop resources");
    embed_resource::compile_for(&resource, [&binary], embed_resource::NONE)
        .manifest_required()
        .expect("embed desktop icon and version");
}
