use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Default)]
struct LinkPlan {
    search_dirs: Vec<PathBuf>,
    fat_static_archives: Vec<PathBuf>,   // Blender-owned .a archives folded into libblender_fat.a
    fat_object_files: Vec<PathBuf>,      // selected .o files folded into libblender_fat.a
    external_static_libs: Vec<PathBuf>,  // third-party .a linked separately by Cargo
    dylib_names: Vec<String>,            // -lfoo seen in dynamic mode
    dylib_paths: Vec<PathBuf>,           // explicit libfoo.so / libfoo.so.X paths
}

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let blender_source_dir =
        env::var("BLENDER_SOURCE_DIR").unwrap_or_else(|_| "/home/bu/dvl/cpp/blender".to_string());

    let blender_build_dir =
        env::var("BLENDER_BUILD_DIR").unwrap_or_else(|_| "/home/bu/dvl/cpp/blender_build".to_string());

    println!("cargo:rerun-if-env-changed=BLENDER_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=BLENDER_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=AR");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=NM");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.cpp");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.h");

    let dst = cmake::Config::new(&crate_dir)
        .define("BLENDER_SOURCE_DIR", &blender_source_dir)
        .define("BLENDER_BUILD_DIR", &blender_build_dir)
        .profile("RelWithDebInfo")
        .build();

    let shim_lib = dst.join("lib").join("libblender_shim.a");
    if !shim_lib.exists() {
        panic!("expected shim archive was not built: {}", shim_lib.display());
    }

    let link_txt =
        Path::new(&blender_build_dir).join("source/creator/CMakeFiles/blender.dir/link.txt");
    println!("cargo:rerun-if-changed={}", link_txt.display());

    let link_line = fs::read_to_string(&link_txt)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", link_txt.display()));

    let tokens = shell_split(&link_line);
    let creator_dir = Path::new(&blender_build_dir).join("source/creator");

    let mut plan = parse_link_plan(&tokens, &creator_dir, Path::new(&blender_build_dir));

    insert_front_unique(&mut plan.fat_static_archives, fs::canonicalize(&shim_lib).unwrap_or(shim_lib));

    inject_required_blender_archives(
        &mut plan.fat_static_archives,
        &mut plan.external_static_libs,
        Path::new(&blender_build_dir),
    );

    remove_fat_archives_from_external(&plan.fat_static_archives, &mut plan.external_static_libs);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_lib_dir = out_dir.join("lib");
    let dynlink_dir = out_dir.join("dynlib-links");
    let staticlink_dir = out_dir.join("staticlib-links");
    let work_dir = out_dir.join("fat-archive-work");

    fs::create_dir_all(&out_lib_dir).unwrap();
    fs::create_dir_all(&dynlink_dir).unwrap();
    fs::create_dir_all(&staticlink_dir).unwrap();

    if work_dir.exists() {
        fs::remove_dir_all(&work_dir).unwrap();
    }
    fs::create_dir_all(&work_dir).unwrap();

    let fat_archive = out_lib_dir.join("libblender_fat.a");
    create_fat_archive(
        &fat_archive,
        &plan.fat_static_archives,
        &plan.fat_object_files,
        &work_dir,
    );

    let mut propagated_search_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut propagated_static_libs: Vec<String> = Vec::new();
    let mut propagated_dylibs: BTreeSet<String> = BTreeSet::new();

    propagated_search_dirs.insert(out_lib_dir.clone());
    propagated_search_dirs.insert(dynlink_dir.clone());
    propagated_search_dirs.insert(staticlink_dir.clone());

    for dir in &plan.search_dirs {
        propagated_search_dirs.insert(dir.clone());
    }

    for lib in ["stdc++", "atomic", "util", "c", "m", "dl", "rt", "pthread"] {
        propagated_dylibs.insert(lib.to_string());
    }

    for lib in &plan.dylib_names {
        if lib != "blender_shim" {
            propagated_dylibs.insert(lib.clone());
        }
    }

    for so_path in &plan.dylib_paths {
        let name = derive_dylib_name(so_path).unwrap_or_else(|| {
            panic!(
                "could not derive dylib name from shared library path: {}",
                so_path.display()
            )
        });

        let local_link = dynlink_dir.join(format!("lib{name}.so"));
        create_or_refresh_symlink(so_path, &local_link);
        propagated_dylibs.insert(name);
    }

    for a_path in &plan.external_static_libs {
        let name = derive_static_lib_name(a_path).unwrap_or_else(|| {
            panic!(
                "could not derive static lib name from static archive path: {}",
                a_path.display()
            )
        });

        let local_copy = staticlink_dir.join(format!("lib{name}.a"));
        fs::copy(a_path, &local_copy).unwrap_or_else(|e| {
            panic!(
                "failed to copy static archive {} -> {}: {e}",
                a_path.display(),
                local_copy.display()
            )
        });

        push_unique_string(&mut propagated_static_libs, name);
    }

    println!("cargo:rustc-link-search=native={}", out_lib_dir.display());
    println!("cargo:rustc-link-lib=static=blender_fat");

    for dir in propagated_search_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    for lib in propagated_static_libs {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    for lib in propagated_dylibs {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }

    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dynlink_dir.display());
}

fn parse_link_plan(tokens: &[String], base_dir: &Path, blender_build_dir: &Path) -> LinkPlan {
    let mut plan = LinkPlan::default();

    let mut seen_search = BTreeSet::new();
    let mut seen_fat_static = BTreeSet::new();
    let mut seen_fat_obj = BTreeSet::new();
    let mut seen_external_static = BTreeSet::new();
    let mut seen_dylib_names = BTreeSet::new();
    let mut seen_dylib_paths = BTreeSet::new();

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
            let resolved = normalize_pathbuf(path, base_dir);
            if seen_search.insert(resolved.clone()) {
                plan.search_dirs.push(resolved);
            }
            continue;
        }

        if let Some(lib) = tok.strip_prefix("-l") {
            if lib == "blender_shim" {
                continue;
            }

            if static_mode {
                if let Some(found) = resolve_static_lib(lib, &plan.search_dirs) {
                    if should_skip_input_path(&found) {
                        continue;
                    }

                    if force_external_static_archive(&found) {
                        if seen_external_static.insert(found.clone()) {
                            plan.external_static_libs.push(found);
                        }
                    } else if is_fat_archive_input(&found, blender_build_dir) {
                        if seen_fat_static.insert(found.clone()) {
                            plan.fat_static_archives.push(found);
                        }
                    } else if seen_external_static.insert(found.clone()) {
                        plan.external_static_libs.push(found);
                    }
                } else {
                    println!("cargo:warning=static -l{lib} could not be resolved in search dirs; skipping");
                }
            } else if seen_dylib_names.insert(lib.to_string()) {
                plan.dylib_names.push(lib.to_string());
            }

            continue;
        }

        if tok.starts_with("-Wl,") {
            continue;
        }

        if tok.starts_with('-') {
            continue;
        }

        if is_linkable_input(tok) {
            let resolved = normalize_pathbuf(tok, base_dir);

            if should_skip_input_path(&resolved) {
                continue;
            }

            if is_shared_input_path(&resolved) {
                if seen_dylib_paths.insert(resolved.clone()) {
                    plan.dylib_paths.push(resolved);
                }
                continue;
            }

            if is_object_file(&resolved) {
                if seen_fat_obj.insert(resolved.clone()) {
                    plan.fat_object_files.push(resolved);
                }
                continue;
            }

            if is_static_archive(&resolved) {
                if force_external_static_archive(&resolved) {
                    if seen_external_static.insert(resolved.clone()) {
                        plan.external_static_libs.push(resolved);
                    }
                } else if is_fat_archive_input(&resolved, blender_build_dir) {
                    if seen_fat_static.insert(resolved.clone()) {
                        plan.fat_static_archives.push(resolved);
                    }
                } else if seen_external_static.insert(resolved.clone()) {
                    plan.external_static_libs.push(resolved);
                }
            }
        }
    }

    plan
}

fn inject_required_blender_archives(
    fat_static_archives: &mut Vec<PathBuf>,
    external_static_libs: &mut Vec<PathBuf>,
    blender_build_dir: &Path,
) {
    let fat_names = [
        "libbf_editor_curves.a",
        "libbf_editor_grease_pencil.a",
        "libbf_editor_sculpt_paint.a",

        "libbf_intern_cycles.a",

        "libcycles_bvh.a",
        "libcycles_device.a",
        "libcycles_graph.a",
        "libcycles_integrator.a",
        "libcycles_kernel_cpu.a",
        "libcycles_kernel_osl.a",
        "libcycles_scene.a",
        "libcycles_session.a",
        "libcycles_subd.a",
        "libcycles_util.a",
    ];

    for name in fat_names {
        if let Some(p) = find_file_recursive(blender_build_dir, name) {
            insert_front_unique(fat_static_archives, p);
        } else {
            println!("cargo:warning=missing required fat archive: {name}");
        }
    }

    let external_names = [
        "libextern_wcwidth.a",
        "libbf_intern_libc_compat.a",
    ];

    for name in external_names {
        if let Some(p) = find_file_recursive(blender_build_dir, name) {
            insert_front_unique(external_static_libs, p);
        } else {
            println!("cargo:warning=missing required external archive: {name}");
        }
    }
}

fn force_external_static_archive(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("libextern_wcwidth.a" | "libbf_intern_libc_compat.a")
    )
}

fn find_file_recursive(root: &Path, needle: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, needle: &str) -> Option<PathBuf> {
        let entries = fs::read_dir(dir).ok()?;
        for entry in entries {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(found) = walk(&path, needle) {
                    return Some(found);
                }
            } else if path.file_name().and_then(|s| s.to_str()) == Some(needle) {
                return Some(fs::canonicalize(&path).unwrap_or(path));
            }
        }
        None
    }

    walk(root, needle)
}

fn is_fat_archive_input(path: &Path, blender_build_dir: &Path) -> bool {
    let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let blender_build_lib = fs::canonicalize(blender_build_dir.join("lib"))
        .unwrap_or_else(|_| blender_build_dir.join("lib"));

    canon.starts_with(&blender_build_lib)
}

fn create_fat_archive(
    output_archive: &Path,
    archives: &[PathBuf],
    object_files: &[PathBuf],
    work_dir: &Path,
) {
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());

    if output_archive.exists() {
        fs::remove_file(output_archive).unwrap();
    }

    let mut existing_archives = Vec::new();
    for archive in archives {
        if archive.exists() {
            existing_archives.push(archive.clone());
        } else {
            println!(
                "cargo:warning=missing fat archive input, skipping: {}",
                archive.display()
            );
        }
    }

    let mut existing_objects = Vec::new();
    for obj in object_files {
        if !obj.exists() {
            println!(
                "cargo:warning=missing fat object input, skipping: {}",
                obj.display()
            );
            continue;
        }

        if should_skip_input_path(obj) {
            continue;
        }

        let fname = obj.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if should_skip_extracted_object(fname) {
            continue;
        }

        existing_objects.push(obj.clone());
    }

    if existing_archives.is_empty() && existing_objects.is_empty() {
        panic!("no inputs collected for fat archive");
    }

    let script_path = work_dir.join("ar.mri");
    let mut script = String::new();

    script.push_str(&format!("CREATE {}\n", output_archive.display()));

    for archive in &existing_archives {
        script.push_str(&format!("ADDLIB {}\n", archive.display()));
    }

    for obj in &existing_objects {
        script.push_str(&format!("ADDMOD {}\n", obj.display()));
    }

    script.push_str("SAVE\nEND\n");

    fs::write(&script_path, script).unwrap_or_else(|e| {
        panic!("failed to write MRI script {}: {e}", script_path.display())
    });

    let mut cmd = Command::new(&ar);
    cmd.current_dir(work_dir).arg("-M");
    run_with_stdin_file(&mut cmd, &script_path);
}

fn run_with_stdin_file(cmd: &mut Command, stdin_path: &Path) {
    let file = File::open(stdin_path).unwrap_or_else(|e| {
        panic!("failed to open stdin file {}: {e}", stdin_path.display())
    });

    let status = cmd
        .stdin(Stdio::from(file))
        .status()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {e}", cmd));

    if !status.success() {
        panic!("command failed with status {status}: {:?}", cmd);
    }
}

fn should_skip_extracted_object(file_name: &str) -> bool {
    matches!(
        file_name,
        "creator.cc.o"
            | "creator_args.cc.o"
            | "creator_signals.cc.o"
            | "shader_tool.cc.o"
    )
}

fn resolve_static_lib(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let file = format!("lib{name}.a");
    search_dirs
        .iter()
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.exists())
}

fn derive_static_lib_name(path: &Path) -> Option<String> {
    let file = path.file_name()?.to_str()?;
    if !file.starts_with("lib") || !file.ends_with(".a") {
        return None;
    }
    Some(file[3..file.len() - 2].to_string())
}

fn derive_dylib_name(path: &Path) -> Option<String> {
    let file = path.file_name()?.to_str()?;
    if !file.starts_with("lib") {
        return None;
    }

    let rest = &file[3..];

    if let Some(name) = rest.strip_suffix(".so") {
        return Some(name.to_string());
    }

    if let Some((name, _)) = rest.split_once(".so.") {
        return Some(name.to_string());
    }

    None
}

fn create_or_refresh_symlink(target: &Path, link: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        if link.exists() || link.symlink_metadata().is_ok() {
            let _ = fs::remove_file(link);
        }

        symlink(target, link).unwrap_or_else(|e| {
            panic!(
                "failed to create symlink {} -> {}: {e}",
                link.display(),
                target.display()
            )
        });
    }

    #[cfg(not(unix))]
    {
        let _ = fs::remove_file(link);
        fs::copy(target, link).unwrap_or_else(|e| {
            panic!(
                "failed to copy shared library {} -> {}: {e}",
                target.display(),
                link.display()
            )
        });
    }
}

fn insert_front_unique(vec: &mut Vec<PathBuf>, item: PathBuf) {
    if let Some(pos) = vec.iter().position(|x| x == &item) {
        vec.remove(pos);
    }
    vec.insert(0, item);
}

fn run(cmd: &mut Command) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run {:?}: {e}", cmd));
    if !status.success() {
        panic!("command failed with status {status}: {:?}", cmd);
    }
}

fn normalize_pathbuf(raw: &str, base_dir: &Path) -> PathBuf {
    let p = Path::new(raw);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    };

    fs::canonicalize(&joined).unwrap_or(joined)
}

fn looks_like_compiler(tok: &str) -> bool {
    let name = Path::new(tok)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(tok);

    matches!(
        name,
        "c++" | "g++" | "clang++" | "x86_64-linux-gnu-g++" | "x86_64-linux-gnu-c++"
    ) || name.starts_with("g++-")
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

fn is_static_archive(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("a")
}

fn is_object_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("o")
}

fn is_shared_input_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.ends_with(".so") || s.contains(".so.")
}

fn should_skip_input_path(path: &Path) -> bool {
    let s = path.to_string_lossy();

    s.ends_with("/bin/blender")
        || s.ends_with("/creator.cc.o")
        || s.ends_with("/creator_args.cc.o")
        || s.ends_with("/creator_signals.cc.o")
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

fn remove_fat_archives_from_external(
    fat_static_archives: &[PathBuf],
    external_static_libs: &mut Vec<PathBuf>,
) {
    let fat_set: BTreeSet<PathBuf> = fat_static_archives
        .iter()
        .map(|p| fs::canonicalize(p).unwrap_or_else(|_| p.clone()))
        .collect();

    external_static_libs.retain(|p| {
        let canon = fs::canonicalize(p).unwrap_or_else(|_| p.clone());
        !fat_set.contains(&canon)
    });
}

fn push_unique_string(vec: &mut Vec<String>, value: String) {
    if !vec.iter().any(|v| v == &value) {
        vec.push(value);
    }
}