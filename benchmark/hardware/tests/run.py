import os, sys, git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *
from frequency_explorer import *
from behavior.tests.run import *


this_dir = os.path.dirname(os.path.abspath(__file__))
run_dir = os.path.join(this_dir, "run")


class HardwareTest(TestVariant):

    def get_graph_builder(self) -> MicroBlossomGraphBuilder:
        config = self.embedded_main_config()
        graph_path = config["graph"]
        graph_folder = os.path.dirname(graph_path)
        graph_file = os.path.basename(graph_path)
        assert graph_file.endswith(".json")
        graph_name = graph_file[:-5].lower()

        return MicroBlossomGraphBuilder(
            graph_folder=graph_folder,
            name=graph_name,
            d=None,
            p=None,
            noisy_measurements=None,
            max_half_weight=None,
        )

    def get_project(self) -> MicroBlossomAxi4Builder:
        config = self.config()
        return MicroBlossomAxi4Builder(
            graph_builder=self.get_graph_builder(),
            name=self.name().lower(),
            project_folder=os.path.join(this_dir, "tmp-project"),
            clock_frequency=100,
            clock_divide_by=config.get("clock_divide_by", 2),
            broadcast_delay=config.get("broadcast_delay", 0),
            convergecast_delay=config.get("convergecast_delay", 1),
            context_depth=config.get("context_depth", 1),
            inject_registers=config.get("inject_registers", ""),
            # ignore bus_type and use_32_bus because the hardware project does not support it
        )

    def get_make_env(self):
        config = self.embedded_main_config()
        make_env = os.environ.copy()
        for key in config:
            if key == "graph":
                continue
            value = config[key]
            make_env[key.upper()] = str(value)
        return make_env

    def run_hardware_test(self):
        project = self.get_project()
        project.create_vivado_project()
        make_env = self.get_make_env()
        project.build_embedded_binary(make_env)
        project.build_vivado_project(force_recompile_binary=True)
        if project.timing_sanity_check_failed():
            exit(1)
        tty_output = project.run_application()


def main():
    compile_code_if_necessary()

    if not os.path.exists(run_dir):
        os.mkdir(run_dir)

    filtered_variants = [
        variant
        for variant in variants
        if "bus_type" not in variant and "use_32_bus" not in variant
    ]

    print(f"There are {len(filtered_variants)} variants...")

    for variant in filtered_variants:

        test = HardwareTest(variant)
        test.run_hardware_test()

        exit(0)


if __name__ == "__main__":
    main()
