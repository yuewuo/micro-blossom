use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Put the linker script somewhere the linker can find it.
fn main() {
    println!("cargo:rerun-if-env-changed=EMBEDDED_BLOSSOM_MAIN");
    println!("cargo:rerun-if-env-changed=MAX_NODE_NUM");
    let out_dir = env::var("OUT_DIR").expect("No out dir");
    let embedded_blossom_main = env::var("EMBEDDED_BLOSSOM_MAIN").unwrap_or_else(|_| "hello_world".to_string());
    let dest_path = Path::new(&out_dir);

    // generate embedded_blossom_main.rs
    std::fs::write(
        &dest_path.join("embedded_blossom_main.name"),
        format!("mains::{embedded_blossom_main}::main()"),
    )
    .unwrap();

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH");
    if target_arch == Ok("riscv32".to_string()) {
        // riscv32 specific settings
        let mut f = File::create(&dest_path.join("riscv-memory.x")).expect("Could not create file");

        f.write_all(include_bytes!("riscv-memory.x")).expect("Could not write file");

        println!("cargo:rustc-link-search={}", dest_path.display());

        println!("cargo:rerun-if-changed=riscv-memory.x");
        println!("cargo:rerun-if-changed=build.rs");
    }
}
