use std::{
    env,
    error::Error,
    ffi::OsStr,
    fs::read_dir,
    path::{Path, PathBuf},
    process::{Command, Output, exit},
    str,
};

const LLVM_MAJOR_VERSION: usize = if cfg!(feature = "llvm16-0") {
    16
} else if cfg!(feature = "llvm17-0") {
    17
} else if cfg!(feature = "llvm18-0") {
    18
} else if cfg!(feature = "llvm19-0") {
    19
} else if cfg!(feature = "llvm20-0") {
    20
} else {
    21
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let llvm_config_path = locate_llvm_config()?;

    let version = llvm_config(&llvm_config_path, "--version")?;
    if !version.starts_with(&format!("{LLVM_MAJOR_VERSION}.")) {
        return Err(format!(
            "failed to find correct version ({LLVM_MAJOR_VERSION}.x.x) of llvm-config (found {version})",
        )
        .into());
    }

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=cc");

    let libdir = llvm_config(&llvm_config_path, "--libdir")?;
    println!("cargo:rustc-link-search={}", libdir);

    build_c_library(&llvm_config_path)?;

    for name in llvm_config(&llvm_config_path, "--libnames")?
        .trim()
        .split(' ')
    {
        println!("cargo:rustc-link-lib=static={}", parse_library_name(name)?);
    }

    for name in get_system_libraries(&llvm_config_path)? {
        println!("cargo:rustc-link-lib={}", name);
    }

    if let Some(name) = get_system_libcpp() {
        println!("cargo:rustc-link-lib={name}");
    }

    bindgen::builder()
        .header("wrapper.h")
        .clang_arg("-Icc/include")
        .clang_arg(format!(
            "-I{}",
            llvm_config(&llvm_config_path, "--includedir")?
        ))
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()?
        .write_to_file(Path::new(&env::var("OUT_DIR")?).join("bindings.rs"))?;

    Ok(())
}

fn locate_llvm_config() -> Result<PathBuf, Box<dyn Error>> {
    let bin_name = if cfg!(target_os = "windows") {
        "llvm-config.exe"
    } else {
        "llvm-config"
    };

    if let Ok(prefix_path) = env::var(format!("TABLEGEN_{LLVM_MAJOR_VERSION}0_PREFIX")) {
        let llvm_config = PathBuf::from(prefix_path).join("bin").join(bin_name);
        if llvm_config.exists() {
            return Ok(llvm_config);
        }
    }

    // Homebrew (macOS)
    if cfg!(target_os = "macos") {
        if let Some(prefix) = homebrew_prefix(&format!("llvm@{}", LLVM_MAJOR_VERSION)) {
            let llvm_config = PathBuf::from(prefix).join("bin").join(bin_name);
            if llvm_config.exists() {
                return Ok(llvm_config);
            }
        }

        if let Some(prefix) = homebrew_prefix("llvm") {
            let llvm_config = PathBuf::from(prefix).join("bin").join(bin_name);
            if llvm_config.exists() {
                return Ok(llvm_config);
            }
        }
    }

    Ok(PathBuf::from(bin_name))
}

fn homebrew_prefix(name: &str) -> Option<String> {
    let output = Command::new("brew").arg("--prefix").arg(name).output();

    output
        .ok()
        .filter(|o| !o.stdout.is_empty())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|val| val.trim().to_string())
}

fn build_c_library(llvm_config_path: &Path) -> Result<(), Box<dyn Error>> {
    let cxxflags = llvm_config(llvm_config_path, "--cxxflags")?;
    let cflags = llvm_config(llvm_config_path, "--cflags")?;
    let includedir = llvm_config(llvm_config_path, "--includedir")?;

    unsafe { env::set_var("CXXFLAGS", cxxflags) };
    unsafe { env::set_var("CFLAGS", cflags) };

    cc::Build::new()
        .cpp(true)
        .files(
            read_dir("cc/lib")?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && path.extension() == Some(OsStr::new("cpp"))),
        )
        .include("cc/include")
        .include(&includedir)
        .flag(if cfg!(target_env = "msvc") {
            "/WX"
        } else {
            "-Werror"
        })
        .std("c++17")
        .compile("CTableGen");

    Ok(())
}

fn get_system_libcpp() -> Option<&'static str> {
    if cfg!(target_env = "msvc") {
        None
    } else if cfg!(target_os = "macos") {
        Some("c++")
    } else {
        Some("stdc++")
    }
}

fn get_system_libraries(llvm_config_path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let output = llvm_config(llvm_config_path, "--system-libs")?;

    let libraries: Vec<String> = output
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .filter_map(|flag| {
            if cfg!(target_env = "msvc") {
                // MSVC: foo.lib
                flag.strip_suffix(".lib").map(|s| s.to_string())
            } else if let Some(lib) = flag.strip_prefix("-l") {
                // Unix linker flags: -lfoo
                if cfg!(target_os = "macos") {
                    // Handle .tbd (text-based stub) files on macOS
                    if let Some(lib) = lib.strip_prefix("lib").and_then(|s| s.strip_suffix(".tbd"))
                    {
                        return Some(lib.to_string());
                    }
                }

                // Handle versioned shared libraries like -lz.so.7.0
                if let Some(i) = lib.find(".so.") {
                    return Some(lib[..i].to_string());
                }

                Some(lib.to_string())
            } else if flag.starts_with('/') {
                let path = Path::new(flag);
                if let Some(parent) = path.parent() {
                    println!("cargo:rustc-link-search={}", parent.display());
                }

                path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| {
                        if let Some((stem, _)) = name.rsplit_once(".a") {
                            stem.strip_prefix("lib")
                        } else {
                            None
                        }
                    })
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(libraries)
}

fn llvm_config(llvm_config_path: &Path, argument: &str) -> Result<String, Box<dyn Error>> {
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(llvm_config_path)
        .arg(argument)
        .arg("--link-static")
        .output()?;

    if !status.success() {
        return Err(format!(
            "llvm-config failed with status: {}\nstderr: {}",
            status,
            str::from_utf8(&stderr)?,
        )
        .into());
    }

    Ok(str::from_utf8(&stdout)?.trim().to_string())
}

fn parse_library_name(name: &str) -> Result<String, Box<dyn Error>> {
    // Linux / macOS
    if let Some(name) = name.strip_prefix("lib").and_then(|n| n.strip_suffix(".a")) {
        return Ok(name.to_string());
    }

    // Windows
    if let Some(name) = name.strip_suffix(".lib") {
        return Ok(name.to_string());
    }

    Err(format!("failed to parse library name: {name}").into())
}
