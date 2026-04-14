use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let blender_source_dir =
        env::var("BLENDER_SOURCE_DIR").unwrap_or_else(|_| "/home/bu/dvl/cpp/blender".to_string());

    let blender_build_dir =
        env::var("BLENDER_BUILD_DIR").unwrap_or_else(|_| "/home/bu/dvl/cpp/blender_build".to_string());

    println!("cargo:rerun-if-env-changed=BLENDER_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=BLENDER_BUILD_DIR");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.cpp");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.h");

    let dst = cmake::Config::new(&crate_dir)
        .define("BLENDER_SOURCE_DIR", &blender_source_dir)
        .define("BLENDER_BUILD_DIR", &blender_build_dir)
        .profile("RelWithDebInfo")
        .build();

    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=blender_shim");
    println!("cargo:rustc-link-lib=dylib=stdc++");

    let link_txt =
        Path::new(&blender_build_dir).join("source/creator/CMakeFiles/blender.dir/link.txt");
    println!("cargo:rerun-if-changed={}", link_txt.display());

    let link_line = fs::read_to_string(&link_txt)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", link_txt.display()));

    let tokens = shell_split(&link_line);

    let link_base_dir = link_txt
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .unwrap();

    emit_link_directives(&tokens, link_base_dir);
}

fn emit_link_directives(tokens: &[String], link_base_dir: &Path) {
    let mut seen_search = BTreeSet::new();
    let mut seen_lib = BTreeSet::new();
    let mut seen_arg = BTreeSet::new();

    let mut static_mode = false;
    let mut skip_next = false;

    for (idx, tok) in tokens.iter().enumerate() {
        if idx == 0 && looks_like_compiler(tok) {
            continue;
        }

        if skip_next {
            skip_next = false;
            continue;
        }

        if tok == "-o" {
            skip_next = true;
            continue;
        }

        if tok == "-Wl,-Bstatic" {
            static_mode = true;
            continue;
        }
        if tok == "-Wl,-Bdynamic" {
            static_mode = false;
            continue;
        }

        if let Some(path) = tok.strip_prefix("-L") {
            let resolved = normalize_path(path, link_base_dir);
            if seen_search.insert(resolved.clone()) {
                println!("cargo:rustc-link-search=native={resolved}");
            }
            continue;
        }

        if let Some(lib) = tok.strip_prefix("-l") {
            if lib == "blender_shim" {
                continue;
            }
            let kind = if static_mode { "static" } else { "dylib" };
            let key = format!("{kind}:{lib}");
            if seen_lib.insert(key) {
                println!("cargo:rustc-link-lib={kind}={lib}");
            }
            continue;
        }

        if tok.starts_with("-Wl,") {
            if should_skip_linker_flag(tok) {
                continue;
            }
            if seen_arg.insert(tok.clone()) {
                println!("cargo:rustc-link-arg={tok}");
            }
            continue;
        }

        if tok.starts_with('-') {
            continue;
        }

        if is_linkable_input(tok) {
            let resolved = normalize_path(tok, link_base_dir);

            if should_skip_input_path(&resolved) {
                continue;
            }

            if seen_arg.insert(resolved.clone()) {
                println!("cargo:rustc-link-arg={resolved}");
            }
        }
    }
}

fn normalize_path(raw: &str, base_dir: &Path) -> String {
    let p = Path::new(raw);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    };

    match fs::canonicalize(&joined) {
        Ok(canon) => canon.display().to_string(),
        Err(_) => joined.display().to_string(),
    }
}

fn looks_like_compiler(tok: &str) -> bool {
    tok.ends_with("/c++")
        || tok.ends_with("/g++")
        || tok.ends_with("/clang++")
        || tok.contains("g++-")
        || tok.contains("clang++")
}

fn is_linkable_input(tok: &str) -> bool {
    tok.ends_with(".a")
        || tok.ends_with(".o")
        || tok.ends_with(".so")
        || tok.contains(".so.")
        || tok.starts_with('/')
        || tok.starts_with("./")
        || tok.starts_with("../")
        || tok.starts_with("CMakeFiles/")
}

fn should_skip_input_path(path: &str) -> bool {
    path.ends_with("/bin/blender")
        || path.ends_with("/creator.cc.o")
        || path.ends_with("/creator_args.cc.o")
        || path.ends_with("/creator_signals.cc.o")
}

fn should_skip_linker_flag(flag: &str) -> bool {
    const SKIP_PREFIXES: &[&str] = &[
        "-Wl,--version-script=",
        "-Wl,--dependency-file=",
        "-Wl,-soname,",
    ];

    const SKIP_EXACT: &[&str] = &[
        "-Wl,--as-needed",
        "-Wl,--gc-sections",
        "-Wl,--start-group",
        "-Wl,--end-group",
        "-Wl,--eh-frame-hdr",
        "-Wl,-z,noexecstack",
        "-Wl,-z,relro,-z,now",
    ];

    SKIP_EXACT.contains(&flag) || SKIP_PREFIXES.iter().any(|p| flag.starts_with(p))
}

fn shell_split(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }

    if !cur.is_empty() {
        out.push(cur);
    }

    out
}