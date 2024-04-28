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

d = 3
frequency = 50
p = 0.001


def hardware_proj_name(offloaded: bool):
    return f"offloaded_{offloaded}".lower()


def hardware_proj_dir(offloaded: bool):
    return os.path.join(hardware_dir, hardware_proj_name(offloaded))


def main():
    global d
    compile_code_if_necessary()

    if not os.path.exists(run_dir):
        os.mkdir(run_dir)

    test_syndrome_count = 100
    syndrome_file_path = os.path.join(run_dir, f"run.syndromes")
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
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."

        # add manual syndromes that corresponds to the single edge errors
        syndromes = SyndromesV1(syndrome_file_path)
        assert syndromes.initializer.vertex_num == d * (d + 1) * (d + 1) // 2
        print(syndromes)
        with open(syndrome_file_path, "a", encoding="utf8") as f:
            for edge_index, weighted_edge in enumerate(
                syndromes.initializer.weighted_edges
            ):
                vertex_1, vertex_2, weight = weighted_edge
                defects = []
                for vertex in [vertex_1, vertex_2]:
                    if vertex not in syndromes.initializer.virtual_vertices:
                        defects.append(vertex)
                syndrome = SyndromePattern(defects, [], [])
                f.write(syndrome.to_json(separators=(",", ":")) + "\n")

    defects_file_path = os.path.join(run_dir, f"run.defects")
    if not os.path.exists(defects_file_path):
        # then generate the defects file
        if not os.path.exists(defects_file_path):
            command = micro_blossom_command() + ["parser"]
            command += [syndrome_file_path]
            command += ["--defects-file", defects_file_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

    print("tesing normal exit on decoding benchmarks")

    for offloaded in [False, True]:
        # first copy defects file to embedded directory
        defects_file_path = os.path.join(run_dir, f"run.defects")
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
        log_file_path = os.path.join(hardware_proj_dir(offloaded), "build.log")
        print(f"building offloaded={offloaded}, log output to {log_file_path}")
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

        # run the program and get output
        log_file_path = os.path.join(run_dir, f"test_offloaded_{offloaded}.log")
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
            assert "[exit]" in tty_output


if __name__ == "__main__":
    main()
