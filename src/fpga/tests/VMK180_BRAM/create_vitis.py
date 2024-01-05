import os
import shutil
import vitis
# see <Vitis_Installation_Dir>/cli/examples for examples

name = "vmk180bram"
cpu_ids = ["r5", "a72"]
cpus = [f"psv_cortex{id}_0" for id in cpu_ids]
workspace = f"./{name}_vitis"

client = vitis.create_client(workspace=workspace)

# create platform from XSA
platform = client.create_platform_component(name="platform", hw=f"./{name}/{name}.xsa", os="standalone", cpu=cpus[0], domain_name=f"standalone_{cpus[0]}")
for cpu in cpus[1:]:
    platform.add_domain(cpu=cpu, os="standalone", name=f"standalone_{cpu}")
status = platform.build()
# print(status)
# print(platform.list_domains())

platform.report()

# create application component
platform_xpfm = client.get_platform("platform")
components = []
for cpu_id, cpu in zip(cpu_ids, cpus):
    component = client.create_app_component(name=f"benchmark_{cpu_id}", platform=platform_xpfm, domain=f"standalone_{cpu}")
    components.append(component)
