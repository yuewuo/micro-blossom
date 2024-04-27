import os
import sys
import subprocess
import shutil
from datetime import datetime

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
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
if True:
    from micro_util import *

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
    from get_ttyoutput import get_ttyoutput
this_dir = os.path.dirname(os.path.abspath(__file__))
hardware_dir = os.path.join(this_dir, "hardware")
run_dir = os.path.join(this_dir, "run")

d_vec = [3, 5, 7, 9]
# d_vec = [3, 5, 7]
max_injections = 4
f_vec = [100, 95, 76, 66]
p = 0.001


def total_rounds(d, p):
    return 1000


def hardware_proj_name(d: int, inj: int):
    return f"d_{d}_inj_{inj}"


def hardware_proj_dir(d: int, inj: int):
    return os.path.join(hardware_dir, hardware_proj_name(d, inj))


def main():
    compile_code_if_necessary()

    if not os.path.exists(run_dir):
        os.mkdir(run_dir)

    test_syndrome_count = 100
    for idx, d in enumerate(d_vec):
        syndrome_file_path = os.path.join(run_dir, f"d_{d}.syndromes")
        if not os.path.exists(syndrome_file_path):
            command = fusion_blossom_qecp_generate_command(
                d=d, p=p, total_rounds=test_syndrome_count, noisy_measurements=d - 1
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
            command += ["--parallel", f"0"]  # use all cores
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

        # then generate the defects file
        defects_file_path = os.path.join(run_dir, f"d_{d}.defects")
        if not os.path.exists(defects_file_path):
            command = micro_blossom_command() + ["parser"]
            command += [syndrome_file_path]
            command += ["--defects-file", defects_file_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

    print("tesing normal exit on decoding benchmarks")

    for d in d_vec:
        for inj in range(max_injections):
            # first copy defects file to embedded directory
            defects_file_path = os.path.join(run_dir, f"d_{d}.defects")
            dest_file_path = os.path.join(embedded_dir, "embedded.defects")
            shutil.copyfile(defects_file_path, dest_file_path)

            # then build the embedded program
            make_env = os.environ.copy()
            make_env["EMBEDDED_BLOSSOM_MAIN"] = "benchmark_decoding"
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

            # rebuild image to include the new program
            log_file_path = os.path.join(hardware_proj_dir(d, inj), "build.log")
            print(f"building d={d}, inj={inj}, log output to {log_file_path}")
            with open(log_file_path, "a") as log:
                process = subprocess.Popen(
                    ["make"],
                    universal_newlines=True,
                    stdout=log.fileno(),
                    stderr=log.fileno(),
                    cwd=hardware_proj_dir(d, inj),
                )
                process.wait()
                assert process.returncode == 0, "synthesis error"

            # run the program and get output
            log_file_path = os.path.join(run_dir, f"test_d_{d}_inj_{inj}.log")
            print(f"testing d={d}, inj={inj}, log output to {log_file_path}")
            with open(log_file_path, "a", encoding="utf8") as log:
                log.write(
                    f"[host_event] [make run_a72 start] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
                )
                tty_output, command_output = get_ttyoutput(
                    command=["make", "run_a72"],
                    cwd=hardware_proj_dir(d, inj),
                    silent=True,
                )
                log.write(
                    f"[host_event] [make run_a72 finish] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
                )
                log.write(f"[host_event] [tty_output]\n")
                log.write(tty_output + "\n")
                log.write(f"[host_event] [command_output]\n")
                log.write(command_output + "\n")
                assert "[exit]" in tty_output


if __name__ == "__main__":
    main()
