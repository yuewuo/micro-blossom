import argparse
import shlex
import subprocess
import sys
import os
import re
import shutil
import git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
template_dir = os.path.join(
    git_root_dir, "src", "fpga", "Xilinx", "VMK180_Micro_Blossom"
)
embedded_dir = os.path.join(git_root_dir, "src", "cpu", "embedded")

SCALA_MICRO_BLOSSOM_COMPILATION_DONE = False


def compile_scala_micro_blossom_if_necessary():
    global SCALA_MICRO_BLOSSOM_COMPILATION_DONE
    if SCALA_MICRO_BLOSSOM_COMPILATION_DONE is False:
        process = subprocess.Popen(
            ["sbt", "assembly"],
            universal_newlines=True,
            stdout=sys.stdout,
            stderr=sys.stderr,
            cwd=git_root_dir,
        )
        process.wait()
        assert process.returncode == 0, "compile has error"
        SCALA_MICRO_BLOSSOM_COMPILATION_DONE = True


def run_verilog_generator(parameters):
    # first compile the Scala library
    compile_scala_micro_blossom_if_necessary()
    # then run the generator
    command = [
        "java",
        "-Xmx64G",
        "-cp",
        os.path.join(git_root_dir, "target/scala-2.12/microblossom.jar"),
        "microblossom.MicroBlossomBusGenerator",
    ] + parameters
    process = subprocess.Popen(
        command,
        universal_newlines=True,
        stdout=sys.stdout,
        stderr=sys.stderr,
        cwd=git_root_dir,
    )
    process.wait()
    assert process.returncode == 0, "error when running the generator"


@dataclass
class MicroBlossomProjectBuilder:
    clock_frequency: float
    path: str
    name: str
    graph: str
    parameters: list[str]
    overwrite: bool
    clock_divide_by: float = 1.0

    def project_dir(self) -> str:
        project_dir = os.path.join(self.path, self.name)
        if not os.path.exists(project_dir):
            os.makedirs(project_dir)
        return project_dir

    def generate_verilog(self):
        verilog_path = os.path.abspath(
            os.path.join(self.project_dir(), f"{self.name}_verilog")
        )
        if not os.path.exists(verilog_path):
            os.makedirs(verilog_path)
        parameters = self.parameters + [
            "--output-dir",
            verilog_path,
            "--graph",
            self.graph,
            "--clock-divide-by",
            f"{self.clock_divide_by}",
        ]
        print(
            "the following parameters will be passed to the Scala main function (microblossom.MicroBlossomBusGenerator):"
        )
        print(f"    {' '.join([shlex.quote(para) for para in parameters])}")
        print("Generating Verilog")
        run_verilog_generator(parameters)

    # if file exists, `update` determines whether to regenerate the file
    def copy_project_files(self, update: bool = False):
        self.copy_common_py(update)
        self.copy_makefile(update)
        self.copy_vitis_py(update)
        self.copy_tcl(update)
        self.copy_c_source(update)

    def copy_common_py(self, update: bool = False):
        with open(os.path.join(template_dir, "common.py"), "r", encoding="utf8") as f:
            common_py = f.read()
            common_py = checked_replace(
                common_py, 'name = "vmk180_micro_blossom"', f'name = "{self.name}"'
            )
            common_py = checked_replace(
                common_py,
                'rust_project = "../../../cpu/embedded"',
                f'rust_project = "{embedded_dir}"',
            )
        if not os.path.exists(os.path.join(self.project_dir(), "common.py")) or update:
            with open(
                os.path.join(self.project_dir(), "common.py"), "w", encoding="utf8"
            ) as f:
                f.write(common_py)

    def copy_makefile(self, update: bool = False):
        with open(os.path.join(template_dir, "Makefile"), "r", encoding="utf8") as f:
            makefile = f.read()
            makefile = checked_replace(
                makefile, "NAME = vmk180_micro_blossom", f"NAME = {self.name}"
            )
            makefile = checked_replace(
                makefile,
                "CLOCK_FREQUENCY ?= 200",
                f"CLOCK_FREQUENCY ?= {self.clock_frequency}",
            )
            makefile = checked_replace(
                makefile,
                "CLOCK_DIVIDE_BY ?= 2",
                f"CLOCK_DIVIDE_BY ?= {self.clock_divide_by}",
            )
        if not os.path.exists(os.path.join(self.project_dir(), "Makefile")) or update:
            with open(
                os.path.join(self.project_dir(), "Makefile"), "w", encoding="utf8"
            ) as f:
                f.write(makefile)

    def copy_vitis_py(self, update: bool = False):
        if (
            not os.path.exists(os.path.join(self.project_dir(), "create_vitis.py"))
            or update
        ):
            shutil.copy2(
                os.path.join(template_dir, "create_vitis.py"),
                os.path.join(self.project_dir(), "create_vitis.py"),
            )

    def copy_tcl(self, update: bool = False):
        for tcl_filename in ["create_vivado.tcl", "reimpl_vivado.tcl"]:
            with open(
                os.path.join(template_dir, tcl_filename), "r", encoding="utf8"
            ) as f:
                vivado_tcl = f.read()
                vivado_tcl = checked_replace(
                    vivado_tcl, "set name vmk180_micro_blossom", f"set name {self.name}"
                )
            if (
                not os.path.exists(os.path.join(self.project_dir(), tcl_filename))
                or update
            ):
                with open(
                    os.path.join(self.project_dir(), tcl_filename), "w", encoding="utf8"
                ) as f:
                    f.write(vivado_tcl)
        with open(
            os.path.join(template_dir, "run_xsdb.tcl"), "r", encoding="utf8"
        ) as f:
            run_xsdb_tcl = f.read()
            run_xsdb_tcl = checked_replace(
                run_xsdb_tcl,
                "set name vmk180_micro_blossom",
                f"set name {self.name}",
            )
        if (
            not os.path.exists(os.path.join(self.project_dir(), "run_xsdb.tcl"))
            or update
        ):
            with open(
                os.path.join(self.project_dir(), "run_xsdb.tcl"), "w", encoding="utf8"
            ) as f:
                f.write(run_xsdb_tcl)

    def copy_c_source(self, update: bool = False):
        if not os.path.exists(os.path.join(self.project_dir(), "src")):
            os.makedirs(os.path.join(self.project_dir(), "src"))
        if (
            not os.path.exists(os.path.join(self.project_dir(), "src", "main.c"))
            or update
        ):
            shutil.copy2(
                os.path.join(template_dir, "src", "main.c"),
                os.path.join(self.project_dir(), "src", "main.c"),
            )
        # src/binding.c
        with open(
            os.path.join(template_dir, "src", "binding.c"), "r", encoding="utf8"
        ) as f:
            binding_c = f.read()
            binding_c = checked_replace(
                binding_c,
                "const float TIMER_FREQUENCY = 200e6; // 200MHz",
                f"const float TIMER_FREQUENCY = {self.clock_frequency}e6; // {self.clock_frequency}MHz",
            )
        if (
            not os.path.exists(os.path.join(self.project_dir(), "src", "binding.c"))
            or update
        ):
            with open(
                os.path.join(self.project_dir(), "src", "binding.c"),
                "w",
                encoding="utf8",
            ) as f:
                f.write(binding_c)

    @staticmethod
    def from_args(args=None, run=True, update=True) -> "MicroBlossomProjectBuilder":
        parser = argparse.ArgumentParser(description="Build Micro Blossom")
        parser.add_argument(
            "-n", "--name", required=True, help="the name of the Vivado project"
        )
        parser.add_argument(
            "-p",
            "--path",
            default=".",
            help="folder of the project, a subfolder will be created with the project name",
        )
        parser.add_argument(
            "-b", "--board", default="VMK180", help="FPGA board, e.g., VMK180"
        )
        parser.add_argument(
            "-c", "--clock-frequency", default="200", help="clock frequency in MHz"
        )
        parser.add_argument(
            "-d", "--clock-divide-by", default="2", help="clock divide by"
        )
        parser.add_argument(
            "-g",
            "--graph",
            required=True,
            help="the graph passed as the argument --graph in MicroBlossomBusGenerator; it also searches in /resources/graphs/",
        )
        parser.add_argument(
            "--overwrite",
            action="store_true",
            help="regenerate the verilog and copy the files from the template directory",
        )
        args, parameters = parser.parse_known_args(args=args)

        print("Configurations:")
        board = args.board
        print(f"board: {board}")
        clock_frequency = float(args.clock_frequency)
        clock_divide_by = float(args.clock_divide_by)
        print(f"clock frequency: {clock_frequency}MHz")
        print(
            f"clock divide by: {clock_divide_by}, slow clock: {clock_frequency/clock_divide_by}/MHz"
        )
        path = args.path
        print(f"path: {path}")
        name = args.name
        print(f"project name: {name}")
        assert re.match(r"^[a-zA-Z0-9_]+$", name), f"invalid project name {name}"
        assert "__" not in name, f"invalid project name {name}"  # requires by Vivado
        graph = args.graph
        if not os.path.exists(graph):
            graph = os.path.join(git_root_dir, "resources", "graphs", graph)
        assert os.path.exists(graph)

        project_builder = MicroBlossomProjectBuilder(
            clock_frequency=clock_frequency,
            clock_divide_by=clock_divide_by,
            path=path,
            name=name,
            graph=graph,
            parameters=parameters,
            overwrite=args.overwrite,
        )
        project_builder.copy_project_files(update)
        if run:
            if not args.overwrite:
                project_dir = project_builder.project_dir()
                if os.path.exists(project_dir):
                    print(
                        f"folder {project_dir} already exists, please use `--overwrite` option to overwrite the existing files"
                    )
                    exit(1)
            project_builder.generate_verilog()
        return project_builder


def main(args=None) -> MicroBlossomProjectBuilder:
    return MicroBlossomProjectBuilder.from_args(args)


def checked_replace(original, old, new):
    assert original.count(old) == 1, f"{old} should appear exactly once, sanity check"
    return original.replace(old, new)


if __name__ == "__main__":
    main()
