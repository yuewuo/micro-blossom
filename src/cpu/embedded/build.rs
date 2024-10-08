use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// https://github.com/rust-lang/cargo/issues/9661#issuecomment-1722358176
fn get_cargo_target_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let profile = std::env::var("PROFILE")?;
    let mut target_dir = None;
    let mut sub_path = out_dir.as_path();
    while let Some(parent) = sub_path.parent() {
        if parent.ends_with(&profile) {
            target_dir = Some(parent);
            break;
        }
        sub_path = parent;
    }
    let target_dir = target_dir.ok_or("not found")?;
    Ok(target_dir.to_path_buf())
}

/// Put the linker script somewhere the linker can find it.
fn main() {
    println!("cargo:rerun-if-env-changed=EMBEDDED_BLOSSOM_MAIN");
    println!("cargo:rerun-if-env-changed=MAX_NODE_NUM");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=embedded.defects");
    println!("cargo:rerun-if-changed=src/binding.rs");

    let out_dir = env::var("OUT_DIR").expect("No out dir");
    let embedded_blossom_main = env::var("EMBEDDED_BLOSSOM_MAIN").unwrap_or_else(|_| "hello_world".to_string());
    println!("cargo:rustc-env=EMBEDDED_BLOSSOM_MAIN_NAME={}", embedded_blossom_main);
    let dest_path = Path::new(&out_dir);

    // generate embedded_blossom_main.rs
    std::fs::write(
        &dest_path.join("embedded_blossom_main.name"),
        format!("mains::{embedded_blossom_main}::main()"),
    )
    .unwrap();

    // create c bindgen
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut cbindgen_config: cbindgen::Config = Default::default();
    cbindgen_config.macro_expansion.bitflags = true;
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(cbindgen_config)
        .with_header("/* DO NOT MODIFY: automatically generated by cbindgen */")
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(get_cargo_target_dir().unwrap().parent().unwrap().join("binding.h"));

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH");
    if target_arch == Ok("riscv32".to_string()) {
        // riscv32 specific settings
        let mut f = File::create(&dest_path.join("riscv-memory.x")).expect("Could not create file");

        f.write_all(include_bytes!("riscv-memory.x")).expect("Could not write file");

        println!("cargo:rustc-link-search={}", dest_path.display());

        println!("cargo:rerun-if-changed=riscv-memory.x");
    }

    // create empty embedded.defects if it doesn't exist
    let defects_file = Path::new("./embedded.defects");
    if !defects_file.exists() {
        std::fs::write(defects_file, [u8::MAX; 4]).unwrap();
    }

    // from test_micro_blossom
    println!("cargo:rerun-if-env-changed=EDGE_0_LEFT");
    println!("cargo:rerun-if-env-changed=EDGE_0_VIRTUAL");
    println!("cargo:rerun-if-env-changed=EDGE_0_WEIGHT");
}
