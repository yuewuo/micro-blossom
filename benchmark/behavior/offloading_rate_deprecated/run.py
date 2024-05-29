"""
batch decoding receives all syndrome data and start decoding
"""

import os
import sys
import subprocess
import json
import git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
if True:
    from micro_util import *

    print(fusion_benchmark_dir)
    sys.path.insert(0, fusion_benchmark_dir)

# useful folders
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists

compile_code_if_necessary()


"""
First generate syndrome data under this folder
"""

# d_vec = [3, 5, 7]  # for debugging script
d_vec = [3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23]
p_vec = [0.0005, 0.001, 0.002, 0.005, 0.01]


def total_rounds(d, p):
    return int(10000 * ((7 / d) ** 3) * (0.001 / p))


primal_dual_config = {
    "dual": {
        "log_instructions": True,
        "sim_config": {"support_offloading": True},
    }
}


for p in p_vec:
    for d in d_vec:
        syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}-p{p}.syndromes")
        if os.path.exists(syndrome_file_path):
            print(
                "[warning] use existing syndrome data (if you think it's stale, delete it and rerun)"
            )
        else:
            command = fusion_blossom_qecp_generate_command(
                d=d, p=p, total_rounds=total_rounds(d, p), noisy_measurements=d
            )
            command += ["--code-type", "rotated-planar-code"]
            command += ["--noise-model", "stim-noise-model"]
            command += [
                "--decoder",
                "fusion",
                "--decoder-config",
                '{"only_stab_z":true,"use_combined_probability":true,"skip_decoding":true}',
            ]
            command += [
                "--debug-print",
                "fusion-blossom-syndrome-file",
                "--fusion-blossom-syndrome-export-filename",
                syndrome_file_path,
            ]
            command += ["--parallel", "0"]  # use all cores
            print(command)
            stdout, returncode = fusion_run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

"""
Run simulations
"""

for p in p_vec:
    data_file = os.path.join(script_dir, f"data_p{p}.txt")
    with open(data_file, "w", encoding="utf8") as f:
        f.write("<d> <total_defects> <offloaded> <offloading_rate>\n")

        for idx, d in enumerate(d_vec):

            syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}-p{p}.syndromes")
            benchmark_profile_path = os.path.join(tmp_dir, f"d{d}-p{p}.profile")
            if os.path.exists(benchmark_profile_path):
                print(
                    "[warning] found existing profile (if you think it's stale, delete it and rerun)"
                )
            else:
                command = micro_blossom_benchmark_command(
                    d=d, p=p, total_rounds=total_rounds(d, p), noisy_measurements=d
                )
                command += ["--code-type", "error-pattern-reader"]
                command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
                command += ["--verifier", "none"]
                command += ["--primal-dual-type", "embedded-comb"]
                command += ["--primal-dual-config", json.dumps(primal_dual_config)]
                command += ["--benchmark-profiler-output", benchmark_profile_path]
                print(command)
                stdout, returncode = run_command_get_stdout(command)
                print("\n" + stdout)
                assert returncode == 0, "command fails..."

            profile = Profile(benchmark_profile_path)
            offloaded = profile.sum_offloaded()
            defect_num = profile.sum_defect_num()
            offloading_rate = 0
            if defect_num > 0:
                offloading_rate = offloaded / defect_num
            print(
                f"d {d}: defect_num {defect_num}, offloaded {offloaded}, offloading_rate: {offloading_rate}"
            )
            f.write(
                "%d %d %d %f\n"
                % (
                    d,
                    defect_num,
                    offloaded,
                    offloading_rate,
                )
            )
