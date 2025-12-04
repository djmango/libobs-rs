use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=headers/wrapper.h");
    println!("cargo:rerun-if-changed=headers/display_capture.h");
    println!("cargo:rerun-if-changed=headers/game_capture.h");
    println!("cargo:rerun-if-changed=headers/vec4.c");
    println!("cargo:rerun-if-changed=headers/window_capture.h");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=LIBOBS_PATH");

    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // For development, you can set LIBOBS_PATH to point to your custom libobs
    if let Ok(path) = env::var("LIBOBS_PATH") {
        println!("cargo:rustc-link-search=native={}", path);

        if target_os == "macos" {
            // Try framework first, fall back to dylib
            println!("cargo:rustc-link-search=framework={}", path);
            println!("cargo:rustc-link-lib=framework=libobs");
        } else {
            println!("cargo:rustc-link-lib=dylib=obs");
        }
    } else if target_family == "windows" {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-search=native={}", manifest_dir);
        println!("cargo:rustc-link-lib=dylib=obs");
    } else if target_os == "macos" {
        // macOS: Link to libobs.framework
        let out_dir = env::var("OUT_DIR").unwrap();
        let target_dir = std::path::Path::new(&out_dir)
            .ancestors()
            .find(|p| {
                p.ends_with("target/debug")
                    || p.ends_with("target/release")
                    || p.file_name().and_then(|f| f.to_str()) == Some("debug")
                    || p.file_name().and_then(|f| f.to_str()) == Some("release")
            })
            .and_then(|p| {
                if p.ends_with("debug") || p.ends_with("release") {
                    Some(p)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")));

        println!("cargo:rustc-link-search=native={}", target_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            target_dir.join("deps").display()
        );
        println!("cargo:rustc-link-search=framework={}", target_dir.display());
        println!(
            "cargo:rustc-link-search=framework={}",
            target_dir.join("deps").display()
        );
        println!("cargo:rustc-link-lib=framework=libobs");

        // Add macOS system frameworks that libobs depends on
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=IOSurface");
        println!("cargo:rustc-link-lib=framework=AudioToolbox");
        println!("cargo:rustc-link-lib=framework=VideoToolbox");

        // Set rpath for dylib loading
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/..");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/..");
    } else if target_os == "linux" {
        // Linux: Try pkg-config first, fall back to just linking if OBS not found
        // This allows CI builds without OBS installed
        let version = "30.0.0";
        match pkg_config::Config::new()
            .atleast_version(version)
            .probe("libobs")
        {
            Ok(_) => {
                // OBS found via pkg-config, linking configured automatically
            }
            Err(_) => {
                // OBS not found - emit link directive and let it fail at runtime if needed
                println!("cargo:warning=libobs not found via pkg-config, using fallback linking");
                println!("cargo:rustc-link-lib=dylib=obs");
            }
        }
    } else {
        // Fallback: assume dynamic libobs available via system linker path
        println!("cargo:rustc-link-lib=dylib=obs");
    }

    // Generate bindings for non-Windows platforms or when explicitly requested
    let feature_generate_bindings = env::var_os("CARGO_FEATURE_GENERATE_BINDINGS").is_some();
    let should_generate_bindings = feature_generate_bindings || target_family != "windows";

    if should_generate_bindings {
        // On Linux, use pre-generated bindings by default (avoids needing OBS headers)
        if target_os == "linux" {
            let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
            let bindings_src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("src")
                .join("bindings_linux.rs");
            if bindings_src.exists() {
                println!("cargo:warning=Using pre-generated Linux bindings");
                std::fs::copy(&bindings_src, out_path.join("bindings.rs"))
                    .expect("Failed to copy pre-generated bindings");
            } else {
                // Fall back to generating if pre-generated don't exist
                generate_bindings(&target_os);
            }
        } else {
            generate_bindings(&target_os);
        }
    }
}

// --- bindings generation ---

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

fn get_ignored_macros() -> IgnoreMacros {
    let mut ignored = HashSet::new();
    ignored.insert("FE_INVALID".into());
    ignored.insert("FE_DIVBYZERO".into());
    ignored.insert("FE_OVERFLOW".into());
    ignored.insert("FE_UNDERFLOW".into());
    ignored.insert("FE_INEXACT".into());
    ignored.insert("FE_TONEAREST".into());
    ignored.insert("FE_DOWNWARD".into());
    ignored.insert("FE_UPWARD".into());
    ignored.insert("FE_TOWARDZERO".into());
    ignored.insert("FP_NORMAL".into());
    ignored.insert("FP_SUBNORMAL".into());
    ignored.insert("FP_ZERO".into());
    ignored.insert("FP_INFINITE".into());
    ignored.insert("FP_NAN".into());
    IgnoreMacros(ignored)
}

fn generate_bindings(target_os: &str) {
    let include_win_bindings = env::var_os("CARGO_FEATURE_INCLUDE_WIN_BINDINGS").is_some();

    let mut builder = bindgen::builder()
        .header("headers/wrapper.h")
        .blocklist_function("^_.*")
        .clang_arg(format!("-I{}", "headers/obs"));

    // macOS: Add Homebrew include paths for simde and other dependencies
    if target_os == "macos" {
        // Apple Silicon Macs
        if std::path::Path::new("/opt/homebrew/include").exists() {
            builder = builder.clang_arg("-I/opt/homebrew/include");
        }
        // Intel Macs
        if std::path::Path::new("/usr/local/include").exists() {
            builder = builder.clang_arg("-I/usr/local/include");
        }
        // Tell simde to not use native SIMD - avoids ARM NEON type alignment issues
        builder = builder.clang_arg("-DSIMDE_NO_NATIVE");
    }

    // Apply previous windows/MSVC blocklists when not Linux and feature not enabled.
    if target_os != "linux" && !include_win_bindings {
        builder = builder
            .blocklist_function("blogva")
            .blocklist_function("^ms_.*")
            .blocklist_file(".*windows\\.h")
            .blocklist_file(".*winuser\\.h")
            .blocklist_file(".*wingdi\\.h")
            .blocklist_file(".*winnt\\.h")
            .blocklist_file(".*winbase\\.h")
            .blocklist_file(".*Windows Kits.*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/][^v].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]v[^a].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]va[^d].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]vad[^e].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]vade[^f].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]vadef[^s].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]vadefs[^.].*")
            .blocklist_file(r".*MSVC.*[\\/]include[\\/]vadefs\.[^h].*");
    }

    let bindings = builder
        .parse_callbacks(Box::new(get_ignored_macros()))
        .derive_copy(true)
        .derive_debug(true)
        .derive_default(false)
        .derive_partialeq(false)
        .derive_eq(false)
        .derive_partialord(false)
        .derive_ord(false)
        .merge_extern_blocks(true)
        .layout_tests(false) // Disable layout tests to avoid SIMD type alignment issues
        .generate()
        .expect("Error generating bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_path.join("bindings.rs");
    let bindings_str = bindings.to_string();

    let processed = bindings_str
        .lines()
        .map(|line| {
            if line.trim().starts_with("#[doc") {
                if let (Some(start), Some(end)) = (line.find('"'), line.rfind('"')) {
                    let doc = &line[start + 1..end];
                    let doc = doc.replace("[", "\\\\[").replace("]", "\\\\]");
                    format!("#[doc = \"{}\"]", doc)
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(&bindings_path, processed).expect("Couldn't write bindings");
}
