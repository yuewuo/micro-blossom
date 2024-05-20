import argparse
import shlex
import subprocess
import sys
import os
import re
import shutil


git_root_dir = (
    subprocess.run(
        "git rev-parse --show-toplevel",
        cwd=os.path.dirname(os.path.abspath(__file__)),
        shell=True,
        check=True,
        capture_output=True,
    )
    .stdout.decode(sys.stdout.encoding)
    .strip(" \r\n")
)
template_dir = os.path.join(git_root_dir, "src", "fpga", "Xilinx", "VMK180_BRAM_speed")
embedded_dir = os.path.join(git_root_dir, "src", "cpu", "embedded")


def main(args=None):
    parser = argparse.ArgumentParser(description="Build BRAM speed test")
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
        "-c", "--clock-frequency", default="100", help="clock frequency in MHz"
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
    print(f"clock frequency: {clock_frequency}MHz")
    path = args.path
    print(f"path: {path}")
    name = args.name
    print(f"project name: {name}")
    assert re.match(r"^[a-zA-Z0-9_-]+$", name), f"invalid project name {name}"
    project_dir = os.path.join(path, name)
    if os.path.exists(project_dir) and not args.overwrite:
        print(
            f"folder {project_dir} already exists, please use `--overwrite` option to overwrite the existing files"
        )
        exit(1)
    if not os.path.exists(project_dir):
        os.makedirs(project_dir)

    print("Copying the project files")
    # common.py
    with open(os.path.join(template_dir, "common.py"), "r", encoding="utf8") as f:
        common_py = f.read()
        common_py = checked_replace(
            common_py, 'name = "vmk180_bram"', f'name = "{name}"'
        )
        common_py = checked_replace(
            common_py,
            'rust_project = "../../../cpu/embedded"',
            f'rust_project = "{embedded_dir}"',
        )
    with open(os.path.join(project_dir, "common.py"), "w", encoding="utf8") as f:
        f.write(common_py)
    # Makefile
    with open(os.path.join(template_dir, "Makefile"), "r", encoding="utf8") as f:
        makefile = f.read()
        makefile = checked_replace(makefile, "NAME = vmk180_bram", f"NAME = {name}")
        makefile = checked_replace(
            makefile, "CLOCK_FREQUENCY ?= 200", f"CLOCK_FREQUENCY ?= {clock_frequency}"
        )
    with open(os.path.join(project_dir, "Makefile"), "w", encoding="utf8") as f:
        f.write(makefile)
    # create_vitis.py
    shutil.copy2(
        os.path.join(template_dir, "create_vitis.py"),
        os.path.join(project_dir, "create_vitis.py"),
    )
    # create_vivado.tcl
    with open(
        os.path.join(template_dir, "create_vivado.tcl"), "r", encoding="utf8"
    ) as f:
        create_vivado_tcl = f.read()
        create_vivado_tcl = checked_replace(
            create_vivado_tcl, "set name vmk180_bram", f"set name {name}"
        )
    with open(
        os.path.join(project_dir, "create_vivado.tcl"), "w", encoding="utf8"
    ) as f:
        f.write(create_vivado_tcl)
    # run_xsdb.tcl
    with open(os.path.join(template_dir, "run_xsdb.tcl"), "r", encoding="utf8") as f:
        run_xsdb_tcl = f.read()
        run_xsdb_tcl = checked_replace(
            run_xsdb_tcl, "set name vmk180_bram", f"set name {name}"
        )
    with open(os.path.join(project_dir, "run_xsdb.tcl"), "w", encoding="utf8") as f:
        f.write(run_xsdb_tcl)
    # src/*.c
    if not os.path.exists(os.path.join(project_dir, "src")):
        os.makedirs(os.path.join(project_dir, "src"))
    shutil.copy2(
        os.path.join(template_dir, "src", "main.c"),
        os.path.join(project_dir, "src", "main.c"),
    )
    shutil.copy2(
        os.path.join(template_dir, "src", "binding.c"),
        os.path.join(project_dir, "src", "binding.c"),
    )


def checked_replace(original, old, new):
    assert original.count(old) == 1, f"{old} should appear exactly once, sanity check"
    return original.replace(old, new)


if __name__ == "__main__":
    main()
