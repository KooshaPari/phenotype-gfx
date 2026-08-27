//! build.rs — auto-generate C/C++ FFI headers via cbindgen.
//!
//! Only runs when the `c_api` feature is enabled.  Requires `cbindgen` to be
//! installed (`cargo install cbindgen`).
//!
//! The generated files land in `include/phenotype_gfx.h` (C) and
//! `include/phenotype_gfx.hpp` (C++).  They are **not** consumed by the
//! Rust build itself — they exist purely for C# P/Invoke consumers.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out_dir = PathBuf::from(&crate_dir).join("include");

    // Create the output directory if it doesn't exist.
    std::fs::create_dir_all(&out_dir).expect("failed to create include/ directory");

    // --- C header ---
    run_cbindgen(&crate_dir, "c", &out_dir.join("phenotype_gfx.h"));

    // --- C++ header ---
    run_cbindgen(&crate_dir, "c++", &out_dir.join("phenotype_gfx.hpp"));

    // Tell Cargo to re-run this script if the binding source or config changes.
    println!("cargo:rerun-if-changed=bindings/c_api.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}

fn run_cbindgen(crate_dir: &str, lang: &str, output: &PathBuf) {
    let status = Command::new("cbindgen")
        .args([
            "--crate",
            "phenotype-gfx",
            "--lang",
            lang,
            "--output",
            output.to_str().expect("output path is not valid UTF-8"),
        ])
        .current_dir(crate_dir)
        .status()
        .expect("failed to execute cbindgen — install it with `cargo install cbindgen`");

    if !status.success() {
        panic!(
            "cbindgen failed for lang={} with exit code {:?}",
            lang,
            status.code()
        );
    }
}
