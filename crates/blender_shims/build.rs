use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let blender_source_dir = env::var("BLENDER_SOURCE_DIR")
        .unwrap_or_else(|_| "/home/bu/dvl/cpp/blender".to_string());

    println!("cargo:rerun-if-env-changed=BLENDER_SOURCE_DIR");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.cc");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.h");

    let dst = cmake::Config::new(&crate_dir)
        .define("BLENDER_SOURCE_DIR", &blender_source_dir)
        .profile("RelWithDebInfo")
        .build();

    let lib_dir = dst.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=blender_shim");
}