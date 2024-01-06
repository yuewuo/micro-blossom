import os
import shutil
import vitis
# see <Vitis_Installation_Dir>/cli/examples for examples

name = "vmk180_bram"

cpu_ids = ["r5", "a72"]
cpus = [f"psv_cortex{id}_0" for id in cpu_ids]
rust_projects = [f"../../../cpu/embedded-{id.upper()}" for id in cpu_ids]
archs = ["armv7r-none-eabihf", "aarch64-unknown-none"]
rust_libnames = [f"libembedded_blossom_{id}" for id in cpu_ids]
workspace = f"./{name}_vitis"
import_files = ["binding.c", "binding.h", "main.c"]

# open or create workspace
if os.path.exists(workspace):
    client = vitis.create_client()
    client.set_workspace(workspace)
else:
    client = vitis.create_client(workspace=workspace)

# create platform from XSA only if not exists
if not client.list_platforms():
    platform = client.create_platform_component(name=name, hw=f"./{name}.xsa", os="standalone", cpu=cpus[0], domain_name=f"standalone_{cpus[0]}")
    for cpu in cpus[1:]:
        platform.add_domain(cpu=cpu, os="standalone", name=f"standalone_{cpu}")
    status = platform.build()
    # print(status)
    # print(platform.list_domains())
    platform.report()

# create application component only if not exists
platform_xpfm = client.get_platform(name)
for cpu_id, cpu, rust_project, arch, rust_libname in zip(cpu_ids, cpus, rust_projects, archs, rust_libnames):
    try:
        component = client.get_component(name=f"benchmark_{cpu_id}")
    except Exception:
        component = client.create_app_component(name=f"benchmark_{cpu_id}", platform=platform_xpfm, domain=f"standalone_{cpu}")
    # import source file and patch the application
    component.import_files(from_loc="./src", files=import_files, dest_dir_in_cmp="src")
    rust_staticlib = os.path.join(os.path.abspath(rust_project), "target", arch, "release", f"{rust_libname}.a")
    assert(os.path.exists(rust_staticlib), f"rust static lib not found at {rust_staticlib}, please compile it")
    component.set_app_config(key="USER_LINK_LIBRARIES", values=rust_staticlib)
    ld_script = component.get_ld_script()
    if cpu_id == "r5":
        ld_script.update_memory_region(name="psv_r5_tcm_ram_0", base_address="0", size="0x40000")  # 256KB
        # avoid going through DRAM for stack and statically allocated objects
        # use lock-step mode (disable split mode) so that all 256KB TCM is available to RPU core 0
        for section in [".stack", ".bss", ".sbss", ".tbss"]:
            ld_script.update_ld_section(section=section, region="psv_r5_tcm_ram_0")
    component.build(target="hw")
