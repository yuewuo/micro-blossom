
name = "vmk180_micro_blossom"
workspace = f"./{name}_vitis"

cpu_ids = ["a72"]
cpus = [f"psv_cortex{id}_0" for id in cpu_ids]

rust_project = "../../../cpu/embedded"
archs = ["aarch64-unknown-none"]

profile = "release"
# profile = "debug"

rust_libname = "libembedded_blossom"

import_files = ["binding.c", "binding.h", "main.c"]
