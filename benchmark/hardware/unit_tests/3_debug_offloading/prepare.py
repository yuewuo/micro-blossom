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

    # first generate the graph config file
    syndrome_file_path = os.path.join(hardware_dir, f"prepare.syndromes")
    if not os.path.exists(syndrome_file_path):
        command = fusion_blossom_qecp_generate_command(
            d=d, p=p, total_rounds=10, noisy_measurements=d - 1
        )
        command += ["--code-type", "rotated-planar-code"]
        command += ["--noise-model", "stim-noise-model"]
        command += [
            "--decoder",
            "fusion",
            "--decoder-config",
            '{"only_stab_z":true,"use_combined_probability":true,"skip_decoding":true,"max_half_weight":7}',
        ]
        command += [
            "--debug-print",
            "fusion-blossom-syndrome-file",
            "--fusion-blossom-syndrome-export-filename",
            syndrome_file_path,
        ]
        command += ["--parallel", f"{STO(0)}"]  # use all cores
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."

    # then generate the graph json
    graph_file_path = os.path.join(hardware_dir, f"prepare.json")
    if not os.path.exists(graph_file_path):
        command = micro_blossom_command() + ["parser"]
        command += [syndrome_file_path]
        command += ["--graph-file", graph_file_path]
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."

    for offloaded in [False, True]:
        # create the hardware project
        if not os.path.exists(hardware_proj_dir(offloaded)):
            parameters = ["--name", hardware_proj_name(offloaded)]
            parameters += ["--path", hardware_dir]
            parameters += ["--clock-frequency", f"{frequency}"]
            parameters += ["--graph", graph_file_path]
            if offloaded:
                parameters += ["--support-offloading"]
            build_micro_blossom_main(parameters)

    # then build hello world application
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
    for offloaded in [False, True]:
        log_file_path = os.path.join(hardware_proj_dir(offloaded), "build.log")
        print(f"building offloaded={offloaded}, log output to {log_file_path}")
        if not os.path.exists(
            os.path.join(
                hardware_proj_dir(offloaded), f"{hardware_proj_name(offloaded)}.xsa"
            )
        ):
            with open(log_file_path, "a") as log:
                process = subprocess.Popen(
                    ["make"],
                    universal_newlines=True,
                    stdout=log.fileno(),
                    stderr=log.fileno(),
                    cwd=hardware_proj_dir(offloaded),
                )
                process.wait()
                assert process.returncode == 0, "synthesis error"

    # check timing reports to make sure there are no negative slacks
    sanity_check_failed = False
    for offloaded in [False, True]:
        vivado = VivadoProject(hardware_proj_dir(offloaded))
        wns = vivado.routed_timing_summery().clk_pl_0_wns
        frequency = vivado.frequency()
        period = 1e-6 / frequency
        new_period = period - wns * 1e-9
        new_frequency = 1 / new_period / 1e6
        if wns < 0:
            # negative slack exists, need to lower the clock frequency
            print(f"offloaded={offloaded} clock frequency too high!!!")
            print(
                f"frequency: {frequency}MHz, wns: {wns}ns, should lower the frequency to {new_frequency}MHz"
            )
            sanity_check_failed = True
        else:
            print(
                f"offloaded={offloaded} wns: {wns}ns, potential new frequency is {new_frequency}MHz"
            )
    if sanity_check_failed:
        exit(1)

    # run the hello world application and run on hardware for sanity check
    for offloaded in [False, True]:
        log_file_path = os.path.join(hardware_proj_dir(offloaded), "make.log")
        print(f"testing offloaded={offloaded}, log output to {log_file_path}")
        with open(log_file_path, "a", encoding="utf8") as log:
            log.write(
                f"[host_event] [make run_a72 start] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            )
            tty_output, command_output = get_ttyoutput(
                command=["make", "run_a72"],
                cwd=hardware_proj_dir(offloaded),
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
