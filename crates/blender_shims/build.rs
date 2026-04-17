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
    println!("cargo:rustc-link-lib=dylib=atomic");
    println!("cargo:rustc-link-lib=dylib=util");
    println!("cargo:rustc-link-lib=dylib=c");
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=rt");
    println!("cargo:rustc-link-lib=dylib=pthread");

    let link_txt =
        Path::new(&blender_build_dir).join("source/creator/CMakeFiles/blender.dir/link.txt");
    println!("cargo:rerun-if-changed={}", link_txt.display());

    let link_line = fs::read_to_string(&link_txt)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", link_txt.display()));

    let tokens = shell_split(&link_line);

    // IMPORTANT:
    // Paths in link.txt are relative to the working dir used for linking,
    // which is .../blender_build/source/creator
    let creator_dir = Path::new(&blender_build_dir).join("source/creator");

    emit_link_directives(&tokens, &creator_dir);

}

fn emit_link_directives(tokens: &[String], base_dir: &Path) {
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

        match tok.as_str() {
            "-Wl,-Bstatic" => {
                static_mode = true;
                continue;
            }
            "-Wl,-Bdynamic" => {
                static_mode = false;
                continue;
            }
            _ => {}
        }

        if let Some(path) = tok.strip_prefix("-L") {
            let resolved = normalize_path(path, base_dir);
            eprintln!("LINK ARG: {resolved} 1");
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
            if should_keep_linker_flag(tok) && seen_arg.insert(tok.clone()) {
                println!("cargo:rustc-link-arg={tok}");
            }
            continue;
        }

        if tok.starts_with('-') {
            continue;
        }

        if is_linkable_input(tok) {
            let resolved = normalize_path(tok, base_dir);
            eprintln!("LINK ARG: {resolved} 2");

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
    let name = Path::new(tok)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(tok);

    matches!(name, "c++" | "g++" | "clang++" | "x86_64-linux-gnu-g++" | "x86_64-linux-gnu-c++")
        || name.starts_with("g++-")
        || name.starts_with("clang++-")
        || name.starts_with("x86_64-linux-gnu-g++-")
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

fn should_keep_linker_flag(flag: &str) -> bool {
    flag.starts_with("-Wl,-rpath,") || flag.starts_with("-Wl,-rpath-link,")
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