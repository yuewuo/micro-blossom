use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Put the linker script somewhere the linker can find it.
fn main() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH");
    if target_arch == Ok("riscv32".to_string()) {
        // riscv32 specific settings
        let out_dir = env::var("OUT_DIR").expect("No out dir");
        let dest_path = Path::new(&out_dir);
        let mut f = File::create(&dest_path.join("riscv-memory.x")).expect("Could not create file");

        f.write_all(include_bytes!("riscv-memory.x")).expect("Could not write file");

        println!("cargo:rustc-link-search={}", dest_path.display());

        println!("cargo:rerun-if-changed=riscv-memory.x");
        println!("cargo:rerun-if-changed=build.rs");

        println!("cargo:rerun-if-env-changed=MAX_NODE_NUM");
    }
}
