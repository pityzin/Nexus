use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn find_cmake() -> PathBuf {
    if let Some(path) = env::var_os("CMAKE") {
        return path.into();
    }
    if let Some(path) = env::var_os("CMAKE_COMMAND") {
        return path.into();
    }
    if let Some(path) = env::split_paths(&env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(if cfg!(windows) { "cmake.exe" } else { "cmake" }))
        .find(|candidate| candidate.is_file())
    {
        return path;
    }
    #[cfg(windows)]
    {
        if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
            let root = Path::new(&program_files).join("Microsoft Visual Studio");
            if let Ok(versions) = std::fs::read_dir(root) {
                for version in versions.flatten() {
                    for edition in ["BuildTools", "Community", "Professional", "Enterprise"] {
                        let candidate = version.path().join(edition).join(
                            "Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe",
                        );
                        if candidate.is_file() {
                            return candidate;
                        }
                    }
                }
            }
        }
    }
    panic!("CMake was not found. Install CMake or set CMAKE to its executable path.")
}

fn run(command: &mut Command, description: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to start {description}: {error}"));
    if !status.success() {
        panic!("{description} failed with {status}");
    }
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut root = manifest
        .join("../../..")
        .canonicalize()
        .expect("resolve repository root");
    #[cfg(windows)]
    if let Some(path) = root.to_string_lossy().strip_prefix(r"\\?\") {
        root = PathBuf::from(path);
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join(if cfg!(windows) {
        "native-build-nmake"
    } else {
        "native-build"
    });
    let cmake = find_cmake();
    println!(
        "cargo:rerun-if-changed={}",
        root.join("CMakeLists.txt").display()
    );
    println!("cargo:rerun-if-changed={}", root.join("core/c").display());
    println!("cargo:rerun-if-changed={}", root.join("core/cpp").display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("core/assembly").display()
    );
    let archive_dir = out.join("lib");
    let mut configure = Command::new(&cmake);
    configure.arg("-S").arg(&root).arg("-B").arg(&out);
    #[cfg(windows)]
    configure
        .args(["-G", "NMake Makefiles"])
        .arg("-DCMAKE_BUILD_TYPE=Release");
    configure.arg(format!("-DNEXUS_ARCHIVE_DIR={}", archive_dir.display()));
    run(&mut configure, "CMake configure");
    run(
        Command::new(&cmake)
            .arg("--build")
            .arg(&out)
            .arg("--config")
            .arg("Release")
            .arg("--target")
            .arg("nexus_native"),
        "native library build",
    );
    println!("cargo:rustc-link-search=native={}", archive_dir.display());
    println!("cargo:rustc-link-lib=static=nexus_native");
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
