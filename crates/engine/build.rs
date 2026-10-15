//! No downloads/builds of upstream. Require the independently verified local artifact.
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../..")
        .canonicalize()
        .unwrap();
    let pin_path = root.join("evidence/ea1/artifact-pin.json");
    let pin: serde_json::Value = serde_json::from_slice(&fs::read(&pin_path).unwrap()).unwrap();
    println!("cargo:rerun-if-changed={}", pin_path.display());
    let verify = |path: &str, digest: &str| {
        let file = root.join(path);
        println!("cargo:rerun-if-changed={}", file.display());
        let actual = format!(
            "{:x}",
            Sha256::digest(fs::read(&file).unwrap_or_else(|_| panic!(
                "Missing pinned artifact {}; run make assessment-bootstrap assessment-build",
                file.display()
            )))
        );
        assert_eq!(actual, digest, "Changed pinned artifact {}", file.display());
        file
    };
    assert_eq!(
        env::var("TARGET").unwrap(),
        "x86_64-unknown-linux-gnu",
        "EA1 artifact is Linux x86_64 only"
    );
    let lib = verify(
        pin["library"].as_str().unwrap(),
        pin["library_sha256"].as_str().unwrap(),
    );
    for name in ["libghostty-vt.so", "libghostty-vt.so.0"] {
        let alias = lib.parent().unwrap().join(name);
        println!("cargo:rerun-if-changed={}", alias.display());
        assert_eq!(
            alias.canonicalize().unwrap(),
            lib.canonicalize().unwrap(),
            "Unexpected library alias"
        );
    }
    let zig = verify(
        pin["zig"].as_str().unwrap(),
        pin["zig_sha256"].as_str().unwrap(),
    );
    for (path, digest) in pin["headers"].as_object().unwrap() {
        verify(path, digest.as_str().unwrap());
    }
    let object = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("boundary.o");
    for file in ["native/boundary.c", "native/boundary.h"] {
        println!("cargo:rerun-if-changed={file}");
    }
    let status = Command::new(zig)
        .args([
            "cc",
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-O1",
            "-g",
            "-fPIC",
            "-c",
            "native/boundary.c",
            "-I",
        ])
        .arg(root.join(".artifacts/engine-assessment/ghostty/include"))
        .arg("-o")
        .arg(&object)
        .env(
            "ZIG_GLOBAL_CACHE_DIR",
            root.join(".cache/engine-assessment/zig-global"),
        )
        .env(
            "ZIG_LOCAL_CACHE_DIR",
            root.join(".cache/engine-assessment/zig-local"),
        )
        .status()
        .expect("Pinned Zig compiler");
    assert!(status.success(), "Public C boundary compile failed");
    println!("cargo:rustc-link-arg={}", object.display());
    println!(
        "cargo:rustc-link-search=native={}",
        lib.parent().unwrap().display()
    );
    println!("cargo:rustc-link-lib=dylib=ghostty-vt");
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        lib.parent().unwrap().display()
    );
}
