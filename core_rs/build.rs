use std::fs;
use std::path::Path;

fn copy_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn main() {
    cxx_build::bridge("src/ffi_bridge.rs")
        .include("include")
        .compile("melyar_core_cxx");

    // Copy generated headers to include directory for CMake
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let out_path = Path::new(&out_dir);
        let cxx_crate = out_path.join("cxxbridge/crate");
        let cxx_inc = out_path.join("cxxbridge/include");
        let dest = Path::new("include");

        let _ = copy_all(&cxx_crate, dest);
        let _ = copy_all(&cxx_inc, dest);
    }

    println!("cargo:rerun-if-changed=src/ffi_bridge.rs");
    println!("cargo:rerun-if-changed=include/bridge.hpp");
}
