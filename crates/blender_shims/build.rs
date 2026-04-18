///Here’s the recap of the main dead ends and how the final approach emerged.
//
// At the start, the goal looked simple: switch from linking lots of Blender pieces directly from Rust to having one native artifact that “contains Blender enough” so downstream Rust code stays manageable.
//
// The first dead end was the most obvious one: turning the shim into a shared library. That ran into the classic non-PIC problem: many Blender static objects were not built as position-independent code, so they could not be cleanly re-emitted into a .so. That made SHARED the wrong direction for this build.
//
// So the next idea was a “fat native wrapper” as a static archive. The first version tried to keep reading Blender’s link.txt and re-emit nearly all of it as Cargo linker arguments. That failed for two reasons:
//
// it produced a gigantic, fragile Rust link line
// Cargo dependency metadata does not propagate raw link-arg style information in a clean reusable way
//
// That was why the plan shifted from “replay Blender’s link line” to “fold Blender-owned static archives into one archive, and only expose a modest set of dynamic/static dependencies to Cargo”.
//
// The next dead end was trying to build one big intermediate relocatable object with g++ -r -Wl,--whole-archive .... That looked attractive, but it immediately hit duplicate symbol errors inside third-party archives, especially FFmpeg/theora pieces. The reason is that --whole-archive forcibly drags in everything, including archives that were never meant to be flattened that way. So the lesson there was: do not whole-archive all third-party stuff.
//
// After that, the build split inputs into categories:
//
// Blender-owned static archives to fold into a fat archive
// third-party static archives to keep external
// shared libraries to propagate as dylibs
//
// That got rid of many duplicates, but then another dead end appeared: duplicate symbol: main. This came from accidentally folding in objects or archives that contained Blender tool/program entrypoints, not just library code. The symptom was Rust test harness main colliding with some object inside the fat archive. That forced explicit skipping of creator/program objects and other non-library pieces.
//
// Once main was gone, the next class of failures was unresolved Blender symbols such as BLI_*, BKE_*, queue helpers, polyfill helpers, and later more specialized things from GPU/Cycles/libmv/editor internals. That revealed another important point: the first categorization of “what belongs in fat archive” was still too narrow. Some Blender-internal libraries that were logically part of Blender code were still left outside, so symbols referenced from fat archives could not resolve. That is why the build gradually learned to include more Blender-built archives such as selected editor libs, cycles libs, and internals.
//
// Another dead end was object-by-object extraction and repacking with ar x followed by rebuilding a new archive from extracted .o files. That solved some ordering issues, but it opened several new problems:
//
// member name collisions
// need to rename many extracted objects
// need to skip problematic objects manually
// very brittle behavior with large archive sets
// continued risk of weird symbol clashes or lost archive semantics
//
// That method was especially unpleasant with Blender/Cycles because the archive graph is large and not especially friendly to naïve flattening.
//
// There was also confusion around libmv. At one point the build tried to pull in libextern_libmv.a, but your build tree did not even contain such a file. Only libbf_intern_libmv.a existed. That exposed a wrong assumption imported from earlier guesses. The fix was to stop inventing a non-existent external libmv archive and rely on the actual archive present in your Blender build.
//
// Another repeated source of trouble was accidentally listing the same archive in both buckets:
//
// in fat archive inputs
// in external static libs
//
// That led to duplicate definitions again. The eventual fix was to make the separation explicit and then remove anything from external_static_libs if it was already chosen for fat_static_archives.
//
// A subtler dead end was trying to reason too much from archive names alone. Blender’s naming is helpful, but not perfect. Some archives that look “external-ish” are really part of Blender’s internal dependency closure, while some built in the Blender tree are better left external for this purpose. That is why the final build has both classification logic and a few explicit overrides.
//
// The final big improvement was replacing the extraction/repack approach with ar -M and MRI script commands like CREATE, ADDLIB, and ADDMOD. That was the turning point. It works better here because:
//
// it combines archives at archive level rather than exploding everything into loose objects
// it preserves object membership more naturally
// it avoids many rename/collision headaches
// it is much more suitable for huge sets of Blender archives
//
// So the final solution is essentially this:
//
// build your local libblender_shim.a with CMake
// parse Blender’s link.txt
// classify inputs into:
// Blender-owned static archives to fold into libblender_fat.a
// object files to add selectively
// external static libs to propagate separately
// shared libraries to propagate as dylibs
// explicitly inject a few Blender archives that the auto-detected set tends to miss
// explicitly avoid creator/program objects
// build libblender_fat.a with ar -M using ADDLIB
// expose that fat archive plus the remaining external libs to Cargo in a much smaller, cleaner way
//
// Why this is hopefully the final solution:
//
// it avoids the PIC/shared-library trap
// it avoids replaying Blender’s monstrous final executable link line
// it avoids whole-archiving third-party dependency forests
// it avoids object-extraction brittleness
// it keeps Cargo-facing metadata relatively sane
//
// So the path was basically:
//
// shared lib → impossible due to non-PIC
// replay whole Blender link line → too fragile
// g++ -r whole-archive everything → duplicate symbols
// split internal vs external archives → better, but incomplete
// object extraction + repack → brittle and still error-prone
// archive-level merge via ar -M → workable solution
///
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Default)]
struct LinkPlan {
    // -L...
    search_dirs: Vec<PathBuf>,

    // Blender-built static archives that will be merged into libblender_fat.a
    fat_static_archives: Vec<PathBuf>,

    // Selected standalone objects that should also be added to libblender_fat.a
    fat_object_files: Vec<PathBuf>,

    // Third-party static archives kept as separate Cargo static libs
    external_static_libs: Vec<PathBuf>,

    // -lfoo observed while linker was in dynamic mode
    dylib_names: Vec<String>,

    // Explicit libfoo.so / libfoo.so.X inputs
    dylib_paths: Vec<PathBuf>,
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
    println!("cargo:rerun-if-env-changed=NM");
    println!("cargo:rerun-if-changed=CMakeLists.txt");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.cpp");
    println!("cargo:rerun-if-changed=src-cpp/blender_shim.h");

    // Build our tiny shim first.
    let dst = cmake::Config::new(&crate_dir)
        .define("BLENDER_SOURCE_DIR", &blender_source_dir)
        .define("BLENDER_BUILD_DIR", &blender_build_dir)
        .profile("RelWithDebInfo")
        .build();

    let shim_lib = canonical_or(dst.join("lib").join("libblender_shim.a"));
    if !shim_lib.exists() {
        panic!("expected shim archive was not built: {}", shim_lib.display());
    }

    // Read Blender's final executable link line and derive a reusable library plan from it.
    let blender_build_dir = PathBuf::from(blender_build_dir);
    let link_txt = blender_build_dir.join("source/creator/CMakeFiles/blender.dir/link.txt");
    println!("cargo:rerun-if-changed={}", link_txt.display());

    let link_line = fs::read_to_string(&link_txt)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", link_txt.display()));
    let tokens = shell_split(&link_line);
    let creator_dir = blender_build_dir.join("source/creator");

    let mut plan = parse_link_plan(&tokens, &creator_dir, &blender_build_dir);

    // Always put our own shim into the fat archive.
    push_front_unique_path(&mut plan.fat_static_archives, shim_lib);

    // Some Blender internal archives are not always discoverable from the parsed line in a way
    // that works for our reduced build, so keep a small explicit allow-list.
    inject_required_blender_archives(
        &mut plan.fat_static_archives,
        &mut plan.external_static_libs,
        &blender_build_dir,
    );

    // Nothing should be both inside the fat archive and linked separately.
    remove_paths(&plan.fat_static_archives, &mut plan.external_static_libs);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_lib_dir = out_dir.join("lib");
    let dynlink_dir = out_dir.join("dynlib-links");
    let staticlink_dir = out_dir.join("staticlib-links");
    let work_dir = out_dir.join("fat-archive-work");

    recreate_dir(&work_dir);
    fs::create_dir_all(&out_lib_dir).unwrap();
    fs::create_dir_all(&dynlink_dir).unwrap();
    fs::create_dir_all(&staticlink_dir).unwrap();

    let fat_archive = out_lib_dir.join("libblender_fat.a");
    create_fat_archive(
        &fat_archive,
        &plan.fat_static_archives,
        &plan.fat_object_files,
        &work_dir,
    );

    emit_cargo_link_instructions(
        &plan,
        &out_lib_dir,
        &dynlink_dir,
        &staticlink_dir,
        &fat_archive,
    );
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
            let resolved = canonical_or(base_dir.join(path));
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
                    if should_skip_path(&found) {
                        continue;
                    }

                    if is_external_static_archive(&found) {
                        push_unique_path_if_new(
                            &mut plan.external_static_libs,
                            &mut seen_external_static,
                            found,
                        );
                    } else if is_blender_build_archive(&found, blender_build_dir) {
                        push_unique_path_if_new(
                            &mut plan.fat_static_archives,
                            &mut seen_fat_static,
                            found,
                        );
                    } else {
                        push_unique_path_if_new(
                            &mut plan.external_static_libs,
                            &mut seen_external_static,
                            found,
                        );
                    }
                } else {
                    println!("cargo:warning=static -l{lib} could not be resolved in search dirs; skipping");
                }
            } else if seen_dylib_names.insert(lib.to_string()) {
                plan.dylib_names.push(lib.to_string());
            }

            continue;
        }

        if tok.starts_with("-Wl,") || tok.starts_with('-') {
            continue;
        }

        if !is_linkable_input(tok) {
            continue;
        }

        let resolved = canonical_or(base_dir.join(tok));

        if should_skip_path(&resolved) {
            continue;
        }

        if is_shared_input_path(&resolved) {
            push_unique_path_if_new(
                &mut plan.dylib_paths,
                &mut seen_dylib_paths,
                resolved,
            );
            continue;
        }

        if is_object_file(&resolved) {
            push_unique_path_if_new(
                &mut plan.fat_object_files,
                &mut seen_fat_obj,
                resolved,
            );
            continue;
        }

        if is_static_archive(&resolved) {
            if is_external_static_archive(&resolved) {
                push_unique_path_if_new(
                    &mut plan.external_static_libs,
                    &mut seen_external_static,
                    resolved,
                );
            } else if is_blender_build_archive(&resolved, blender_build_dir) {
                push_unique_path_if_new(
                    &mut plan.fat_static_archives,
                    &mut seen_fat_static,
                    resolved,
                );
            } else {
                push_unique_path_if_new(
                    &mut plan.external_static_libs,
                    &mut seen_external_static,
                    resolved,
                );
            }
        }
    }

    plan
}

fn emit_cargo_link_instructions(
    plan: &LinkPlan,
    out_lib_dir: &Path,
    dynlink_dir: &Path,
    staticlink_dir: &Path,
    _fat_archive: &Path,
) {
    let mut search_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut static_libs: Vec<String> = Vec::new();
    let mut dylibs: BTreeSet<String> = BTreeSet::new();

    search_dirs.insert(out_lib_dir.to_path_buf());
    search_dirs.insert(dynlink_dir.to_path_buf());
    search_dirs.insert(staticlink_dir.to_path_buf());

    for dir in &plan.search_dirs {
        search_dirs.insert(dir.clone());
    }

    for lib in ["stdc++", "atomic", "util", "c", "m", "dl", "rt", "pthread"] {
        dylibs.insert(lib.to_string());
    }

    for lib in &plan.dylib_names {
        if lib != "blender_shim" {
            dylibs.insert(lib.clone());
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
        dylibs.insert(name);
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

        push_unique_string(&mut static_libs, name);
    }

    println!("cargo:rustc-link-search=native={}", out_lib_dir.display());
    println!("cargo:rustc-link-lib=static=blender_fat");

    for dir in search_dirs {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }

    for lib in static_libs {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    for lib in dylibs {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }

    // The symlink directory keeps runtime paths short and stable.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dynlink_dir.display());
}

fn inject_required_blender_archives(
    fat_static_archives: &mut Vec<PathBuf>,
    external_static_libs: &mut Vec<PathBuf>,
    blender_build_dir: &Path,
) {
    // Small explicit list for pieces that are needed but not always convenient to infer.
    let required_fat = [
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

    for name in required_fat {
        if let Some(path) = find_file_recursive(blender_build_dir, name) {
            push_front_unique_path(fat_static_archives, path);
        } else {
            println!("cargo:warning=missing required fat archive: {name}");
        }
    }

    let required_external = [
        "libextern_wcwidth.a",
        "libbf_intern_libc_compat.a",
    ];

    for name in required_external {
        if let Some(path) = find_file_recursive(blender_build_dir, name) {
            push_front_unique_path(external_static_libs, path);
        } else {
            println!("cargo:warning=missing required external archive: {name}");
        }
    }
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

    let existing_archives: Vec<PathBuf> = archives
        .iter()
        .filter(|p| p.exists())
        .cloned()
        .collect();

    for archive in archives {
        if !archive.exists() {
            println!("cargo:warning=missing fat archive input, skipping: {}", archive.display());
        }
    }

    let existing_objects: Vec<PathBuf> = object_files
        .iter()
        .filter(|p| p.exists())
        .filter(|p| !should_skip_path(p))
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            !should_skip_object_name(name)
        })
        .cloned()
        .collect();

    for obj in object_files {
        if !obj.exists() {
            println!("cargo:warning=missing fat object input, skipping: {}", obj.display());
        }
    }

    if existing_archives.is_empty() && existing_objects.is_empty() {
        panic!("no inputs collected for fat archive");
    }

    // MRI script is a simple way to merge many archives without exploding them into loose objects.
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

fn find_file_recursive(root: &Path, needle: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, needle: &str) -> Option<PathBuf> {
        for entry in fs::read_dir(dir).ok()? {
            let path = entry.ok()?.path();

            if path.is_dir() {
                if let Some(found) = walk(&path, needle) {
                    return Some(found);
                }
            } else if path.file_name().and_then(|s| s.to_str()) == Some(needle) {
                return Some(canonical_or(path));
            }
        }
        None
    }

    walk(root, needle)
}

fn resolve_static_lib(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let file = format!("lib{name}.a");
    search_dirs
        .iter()
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.exists())
        .map(canonical_or)
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

fn is_blender_build_archive(path: &Path, blender_build_dir: &Path) -> bool {
    let canon = canonical_or(path.to_path_buf());
    let blender_build_lib = canonical_or(blender_build_dir.join("lib"));
    canon.starts_with(&blender_build_lib)
}

fn is_external_static_archive(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("libextern_wcwidth.a" | "libbf_intern_libc_compat.a")
    )
}

fn should_skip_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.ends_with("/bin/blender")
        || s.ends_with("/creator.cc.o")
        || s.ends_with("/creator_args.cc.o")
        || s.ends_with("/creator_signals.cc.o")
}

fn should_skip_object_name(file_name: &str) -> bool {
    matches!(
        file_name,
        "creator.cc.o" | "creator_args.cc.o" | "creator_signals.cc.o" | "shader_tool.cc.o"
    )
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

fn recreate_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();
}

fn canonical_or(path: PathBuf) -> PathBuf {
    fs::canonicalize(&path).unwrap_or(path)
}

fn push_front_unique_path(vec: &mut Vec<PathBuf>, item: PathBuf) {
    if let Some(pos) = vec.iter().position(|x| x == &item) {
        vec.remove(pos);
    }
    vec.insert(0, item);
}

fn push_unique_path_if_new(
    vec: &mut Vec<PathBuf>,
    seen: &mut BTreeSet<PathBuf>,
    item: PathBuf,
) {
    if seen.insert(item.clone()) {
        vec.push(item);
    }
}

fn remove_paths(remove: &[PathBuf], from: &mut Vec<PathBuf>) {
    let remove_set: BTreeSet<PathBuf> = remove.iter().cloned().map(canonical_or).collect();
    from.retain(|p| !remove_set.contains(&canonical_or(p.clone())));
}

fn push_unique_string(vec: &mut Vec<String>, value: String) {
    if !vec.iter().any(|v| v == &value) {
        vec.push(value);
    }
}