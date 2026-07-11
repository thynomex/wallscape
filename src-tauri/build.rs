fn main() {
    let project_root = project_root();
    let mpv_dir = resolve_mpv_dir(&project_root);
    let mpv_dll = mpv_dir.join("libmpv-2.dll");
    let staged_mpv_dll = std::path::PathBuf::from("bundle/libmpv-2.dll");

    println!("cargo:rerun-if-env-changed=MPV_DEV_DIR");
    println!(
        "cargo:rerun-if-changed={}",
        project_root
            .join(".vendor")
            .join("mpv-dev")
            .join("libmpv-2.dll")
            .display()
    );
    println!("cargo:rerun-if-changed=C:/mpv-dev/libmpv-2.dll");
    println!("cargo:rerun-if-changed={}", mpv_dll.display());
    println!("cargo:rerun-if-changed={}", staged_mpv_dll.display());
    println!("cargo:rustc-link-search=native={}", mpv_dir.display());

    assert_mpv_dev_dir(&mpv_dir);
    stage_mpv_dll_for_bundlers(&mpv_dll, &staged_mpv_dll);
    copy_mpv_dll_to_target_dir(&mpv_dll);

    tauri_build::build()
}

fn project_root() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()),
    );

    manifest_dir.parent().unwrap_or(&manifest_dir).to_path_buf()
}

fn resolve_mpv_dir(project_root: &std::path::Path) -> std::path::PathBuf {
    if let Ok(mpv_dir) = std::env::var("MPV_DEV_DIR") {
        return std::path::PathBuf::from(mpv_dir);
    }

    let vendored_mpv_dir = project_root.join(".vendor").join("mpv-dev");
    if vendored_mpv_dir.join("libmpv-2.dll").is_file() {
        return vendored_mpv_dir;
    }

    std::path::PathBuf::from("C:/mpv-dev")
}

fn assert_mpv_dev_dir(mpv_dir: &std::path::Path) {
    let mpv_dll = mpv_dir.join("libmpv-2.dll");
    let has_import_lib =
        mpv_dir.join("mpv.lib").is_file() || mpv_dir.join("libmpv.dll.a").is_file();

    if mpv_dll.is_file() && has_import_lib {
        return;
    }

    panic!(
        "libmpv dev files were not found in {}. Run `npm run setup:mpv` or set MPV_DEV_DIR to a folder containing libmpv-2.dll and an import library.",
        mpv_dir.display()
    );
}

fn stage_mpv_dll_for_bundlers(mpv_dll: &std::path::Path, staged_mpv_dll: &std::path::Path) {
    if let Err(error) = copy_file_if_changed(mpv_dll, staged_mpv_dll) {
        println!(
            "cargo:warning=Failed to stage {} for installers at {}: {}",
            mpv_dll.display(),
            staged_mpv_dll.display(),
            error
        );
    }
}

fn copy_mpv_dll_to_target_dir(mpv_dll: &std::path::Path) {
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };

    let out_dir = std::path::PathBuf::from(out_dir);
    let Some(target_profile_dir) = out_dir.ancestors().nth(3) else {
        println!("cargo:warning=Could not locate Cargo target profile directory from OUT_DIR");
        return;
    };

    let target_dll = target_profile_dir.join("libmpv-2.dll");

    if let Err(error) = copy_file_if_changed(mpv_dll, &target_dll) {
        println!(
            "cargo:warning=Failed to copy {} to {}: {}",
            mpv_dll.display(),
            target_dll.display(),
            error
        );
    }
}

fn copy_file_if_changed(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    if same_file_contents(source, target)? {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(source, target)?;
    Ok(())
}

fn same_file_contents(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<bool> {
    let source_metadata = std::fs::metadata(source)?;
    let Ok(target_metadata) = std::fs::metadata(target) else {
        return Ok(false);
    };

    if source_metadata.len() != target_metadata.len() {
        return Ok(false);
    }

    Ok(std::fs::read(source)? == std::fs::read(target)?)
}
