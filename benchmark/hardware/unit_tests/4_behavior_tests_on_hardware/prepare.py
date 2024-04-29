import os
import sys
import subprocess
from datetime import datetime
from run import *
from build_micro_blossom import main as build_micro_blossom_main
from get_ttyoutput import get_ttyoutput
from slurm_distribute import slurm_threads_or as STO
from vivado_project import VivadoProject


def main():
    global frequency
    compile_code_if_necessary()

    if not os.path.exists(hardware_dir):
        os.mkdir(hardware_dir)

    # generate names and expanded configurations
    states = []

    for variant in variants:
        config = {"graph": default_graph, **variant}
        if "use_32_bus" in config or "bus_type" in config:
            # do not test for bus type: only Axi4 is supported
            continue

        name = ""
        for key in config:
            value = config[key]
            if key == "graph":
                if value != default_graph:
                    name += "_graph_" + os.path.basename(config["graph"]).split(".")[0]
            else:
                name += f"_{key}_{value}".replace(",", "_")
        name = name.lower()  # vitis doesn't seem to work with upper case name
        if name == "":
            name = "default_config"
        else:
            name = name[1:]  # remove leading _

        graph_file_path = config["graph"]
        assert os.path.exists(graph_file_path)

        states.append((name, config))

    for var_idx, (name, config) in enumerate(states):

        if not os.path.exists(os.path.join(hardware_dir, name)):
            parameters = ["--name", name]
            parameters += ["--path", hardware_dir]
            parameters += ["--clock-frequency", f"{frequency}"]
            parameters += ["--graph", config["graph"]]
            for key in config:
                value = config[key]
                if key == "graph":
                    continue
                if key == "inject_registers":
                    stages = value.split(",")
                    parameters += ["--inject-registers"] + stages
                    continue
                if isinstance(value, bool):
                    parameters += [f"--{key.replace('_', '-')}"]
                else:
                    parameters += [f"--{key.replace('_', '-')}", str(value)]
            print(parameters)
            build_micro_blossom_main(parameters)

        left, virtual, weight = find_edge_0(config["graph"])
        config["EDGE_0_LEFT"] = left
        config["EDGE_0_VIRTUAL"] = virtual
        config["EDGE_0_WEIGHT"] = weight

    # then build hello world application for basic testing
    make_env = os.environ.copy()
    make_env["EMBEDDED_BLOSSOM_MAIN"] = "hello_world"
    process = subprocess.Popen(
        ["make", "Xilinx"],
        universal_newlines=True,
        stdout=sys.stdout,
        stderr=sys.stderr,
        cwd=embedded_dir,
        env=make_env,
    )
    process.wait()
    assert process.returncode == 0, "compile error"

    # build all hardware projects using the hello world application
    for name, config in states:
        hardware_proj_dir = os.path.join(hardware_dir, name)
        log_file_path = os.path.join(hardware_proj_dir, "build.log")
        print(f"building name={name}, log output to {log_file_path}")
        if not os.path.exists(os.path.join(hardware_proj_dir, f"{name}.xsa")):
            with open(log_file_path, "a") as log:
                process = subprocess.Popen(
                    ["make"],
                    universal_newlines=True,
                    stdout=log.fileno(),
                    stderr=log.fileno(),
                    cwd=hardware_proj_dir,
                )
                process.wait()
                assert process.returncode == 0, "synthesis error"

    # check timing reports to make sure there are no negative slacks
    sanity_check_failed = False
    for name, config in states:
        hardware_proj_dir = os.path.join(hardware_dir, name)
        vivado = VivadoProject(hardware_proj_dir)
        wns = vivado.routed_timing_summery().clk_pl_0_wns
        frequency = vivado.frequency()
        period = 1e-6 / frequency
        new_period = period - wns * 1e-9
        new_frequency = 1 / new_period / 1e6
        if wns < 0:
            # negative slack exists, need to lower the clock frequency
            print(f"name={name} clock frequency too high!!!")
            print(
                f"frequency: {frequency}MHz, wns: {wns}ns, should lower the frequency to {new_frequency}MHz"
            )
            sanity_check_failed = True
        else:
            print(
                f"name={name} wns: {wns}ns, potential new frequency is {new_frequency}MHz"
            )
    if sanity_check_failed:
        exit(1)

    # run the hello world application and run on hardware for sanity check
    for name, config in states:
        hardware_proj_dir = os.path.join(hardware_dir, name)
        log_file_path = os.path.join(hardware_proj_dir, "make.log")
        print(f"testing name={name}, log output to {log_file_path}")
        with open(log_file_path, "a", encoding="utf8") as log:
            log.write(
                f"[host_event] [make run_a72 start] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            )
            tty_output, command_output = get_ttyoutput(
                command=["make", "run_a72"],
                cwd=hardware_proj_dir,
                silent=True,
            )
            log.write(
                f"[host_event] [make run_a72 finish] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            )
            log.write(f"[host_event] [tty_output]\n")
            log.write(tty_output + "\n")
            log.write(f"[host_event] [command_output]\n")
            log.write(command_output + "\n")
            assert "Hello world!" in tty_output


if __name__ == "__main__":
    main()
