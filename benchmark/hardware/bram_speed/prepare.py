import os
import sys
import subprocess
from datetime import datetime
from run import *
from build_bram_speed import main as build_bram_speed
from get_ttyoutput import get_ttyoutput
from slurm_distribute import slurm_threads_or as STO
from vivado_project import VivadoProject


def main():
    compile_code_if_necessary()

    if not os.path.exists(hardware_dir):
        os.mkdir(hardware_dir)

    for frequency in enumerate(f_vec):
        # create the hardware project
        if not os.path.exists(hardware_proj_dir(frequency)):
            parameters = ["--name", hardware_proj_name(frequency)]
            parameters += ["--path", hardware_dir]
            parameters += ["--clock-frequency", f"{frequency}"]
            build_bram_speed(parameters)

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
    for frequency in enumerate(f_vec):
        log_file_path = os.path.join(hardware_proj_dir(frequency), "build.log")
        print(f"building frequency={frequency}, log output to {log_file_path}")
        if not os.path.exists(
            os.path.join(
                hardware_proj_dir(frequency), f"{hardware_proj_name(frequency)}.xsa"
            )
        ):
            with open(log_file_path, "a") as log:
                process = subprocess.Popen(
                    ["make"],
                    universal_newlines=True,
                    stdout=log.fileno(),
                    stderr=log.fileno(),
                    cwd=hardware_proj_dir(frequency),
                )
                process.wait()
                assert process.returncode == 0, "synthesis error"

    # check timing reports to make sure there are no negative slacks
    sanity_check_failed = False
    for frequency in enumerate(f_vec):
        vivado = VivadoProject(hardware_proj_dir(frequency))
        wns = vivado.routed_timing_summery().clk_pl_0_wns
        frequency = vivado.frequency()
        period = 1e-6 / frequency
        new_period = period - wns * 1e-9
        new_frequency = 1 / new_period / 1e6
        if wns < 0:
            # negative slack exists, need to lower the clock frequency
            print(f"frequency={frequency} clock frequency too high!!!")
            print(
                f"frequency: {frequency}MHz, wns: {wns}ns, should lower the frequency to {new_frequency}MHz"
            )
            sanity_check_failed = True
        else:
            print(
                f"frequency={frequency} wns: {wns}ns, potential new frequency is {new_frequency}MHz"
            )
    assert not sanity_check_failed

    # run the hello world application and run on hardware for sanity check
    for frequency in enumerate(f_vec):
        log_file_path = os.path.join(hardware_proj_dir(frequency), "make.log")
        print(f"testing frequency={frequency}, log output to {log_file_path}")
        with open(log_file_path, "a", encoding="utf8") as log:
            log.write(
                f"[host_event] [make run_a72 start] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            )
            tty_output, command_output = get_ttyoutput(
                command=["make", "run_a72"],
                cwd=hardware_proj_dir(frequency),
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
