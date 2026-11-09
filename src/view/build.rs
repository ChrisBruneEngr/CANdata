fn main() {
    // CANdata is Windows-only; fail early with a clear message elsewhere.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        panic!("CANdata only builds for Windows targets");
    }

    // Embed the application icon and version metadata in the .exe.
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let icon_path = manifest_dir.join("../../assets/ico/candata.ico");
    let icon_path_str = icon_path.to_str().expect("Invalid icon path");
    println!("cargo:rerun-if-changed={}", icon_path.display());

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path_str);
    if let Err(e) = res.compile() {
        eprintln!("Failed to compile resources: {}", e);
    }

    // Release builds use the GUI subsystem so no console window appears.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg-bins=/ENTRY:mainCRTStartup");
    }

    emit_version();
}

/// Emit CANDATA_VERSION env var from `git describe --tags --always --dirty`.
/// Falls back to CARGO_PKG_VERSION when git is unavailable or no tags exist.
fn emit_version() {
    // Re-run when HEAD moves so the version stays accurate.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/tags");

    let version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        // Tags look like "v0.2.6"; the UI adds its own "v", so drop the tag's.
        .map(|s| s.trim().trim_start_matches('v').to_string())
        .unwrap_or_else(|| {
            // Fall back to Cargo package version when git is unavailable or no tags.
            env!("CARGO_PKG_VERSION").to_string()
        });

    println!("cargo:rustc-env=CANDATA_VERSION={}", version);
}
