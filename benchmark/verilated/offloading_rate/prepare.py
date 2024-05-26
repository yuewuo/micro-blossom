import os
import sys
import subprocess
import sys

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
if True:
    from micro_util import *


# useful folders
script_dir = os.path.dirname(__file__)
syndrome_dir = os.path.join(script_dir, "syndrome")
profile_dir = os.path.join(script_dir, "profile")
os.makedirs(syndrome_dir, exist_ok=True)
os.makedirs(profile_dir, exist_ok=True)

compile_code_if_necessary()


"""
First generate syndrome data under this folder
"""

d_vec = [3, 5, 7]
p_vec = [0.0005, 0.001, 0.0015, 0.002, 0.005, 0.01]
p_vec.reverse()


def total_rounds(d, p):
    return int(10000 * ((7 / d) ** 3) * (0.001 / p))


def prepare():

    for p in p_vec:
        for d in d_vec:
            syndrome_file_path = os.path.join(
                syndrome_dir, f"generated-d{d}-p{p}.syndromes"
            )
            if os.path.exists(syndrome_file_path):
                print(
                    "[warning] use existing syndrome data (if you think it's stale, delete it and rerun)"
                )
            else:
                command = fusion_blossom_qecp_generate_command(
                    d=d,
                    p=p,
                    total_rounds=total_rounds(d, p),
                    noisy_measurements=d - 1,
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
                print(" ".join(command))

                stdout, returncode = run_command_get_stdout(command)
                print("\n" + stdout)
                assert returncode == 0, "command fails..."


def run(name: str, primal_dual_type: str, primal_dual_config: any):
    for p in p_vec:
        filename = os.path.join(script_dir, f"{name}_p{p}.txt")
        results = []

        for idx, d in enumerate(d_vec):

            syndrome_file_path = os.path.join(
                syndrome_dir, f"generated-d{d}-p{p}.syndromes"
            )
            benchmark_profile_path = os.path.join(
                profile_dir, f"{name}-d{d}-p{p}.profile"
            )
            if os.path.exists(benchmark_profile_path):
                print(
                    "[warning] found existing profile (if you think it's stale, delete it and rerun)"
                )
            else:
                command = micro_blossom_benchmark_command(
                    d=d,
                    p=p,
                    total_rounds=total_rounds(d, p),
                    noisy_measurements=d - 1,
                )
                command += ["--code-type", "error-pattern-reader"]
                command += [
                    "--code-config",
                    f'{{"filename":"{syndrome_file_path}"}}',
                ]
                command += ["--verifier", "fusion-serial"]
                command += ["--primal-dual-type", primal_dual_type]
                command += ["--primal-dual-config", json.dumps(primal_dual_config)]
                command += ["--benchmark-profiler-output", benchmark_profile_path]
                print(" ".join(command))

                stdout, returncode = run_command_get_stdout(command)
                print("\n" + stdout)
                assert returncode == 0, "command fails..."

            print(benchmark_profile_path)
            profile = Profile(benchmark_profile_path)
            offloaded = profile.sum_offloaded()
            defect_num = profile.sum_defect_num()
            offloading_rate = 0
            if defect_num > 0:
                offloading_rate = offloaded / defect_num
            confidence_interval = (
                math.sqrt(
                    1.96 * (offloading_rate * (1.0 - offloading_rate) / defect_num)
                )
                / offloading_rate
            )
            print(
                f"d {d}: defect_num {defect_num}, offloaded {offloaded}, offloading_rate: {offloading_rate}, confidence: {confidence_interval}"
            )
            print_result = "%d %d %d %f %.2e" % (
                d,
                defect_num,
                offloaded,
                offloading_rate,
                confidence_interval,
            )
            results.append(print_result)

        print("\n\n")
        print("\n".join(results))
        print("\n\n")

        with open(filename, "w", encoding="utf8") as f:
            f.write("<d> <total_defects> <offloaded> <offloading_rate>\n")
            f.write("\n".join(results) + "\n")


if __name__ == "__main__":
    prepare()
