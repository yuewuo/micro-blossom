"""
batch decoding receives all syndrome data and start decoding
"""

import os
import sys
import subprocess
import sys
import git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
if True:
    from micro_util import *

    print(fusion_benchmark_dir)
    sys.path.insert(0, fusion_benchmark_dir)
    import slurm_distribute
    from slurm_distribute import slurm_threads_or as STO
    from slurm_distribute import cpu_hours as CH


slurm_distribute.SLURM_DISTRIBUTE_TIME = "10:20:00"
slurm_distribute.SLURM_DISTRIBUTE_MEM_PER_TASK = "8G"
# for more usuable machines, use `SLURM_USE_SCAVENGE_PARTITION=1` flag
slurm_distribute.SLURM_DISTRIBUTE_CPUS_PER_TASK = 1


# useful folders
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists

compile_code_if_necessary()


"""
First generate syndrome data under this folder
"""

p_per_10 = 5
p_vec = [1e-4 * (10 ** (i / p_per_10)) for i in range(-1, p_per_10 * 2 + 1)]
# d_vec = [3, 5, 7]  # for debugging script
d_vec = [3, 5, 7, 9, 11, 13, 15]
# use the same physical error rate to construct the decoder for consistence with the hardware evaluation
p_graph = 0.001


def total_rounds(d, p):
    return int(100000 * ((7 / d) ** 3) * (0.001 / p))


evaluation_vec = [
    (
        "pre_match",
        {
            "dual": {
                # "log_instructions": True,
                "sim_config": {"support_offloading": True},
            }
        },
    ),
    (
        "layer_fusion",
        {
            "dual": {
                # "log_instructions": True,
                "sim_config": {
                    "support_offloading": True,
                    "support_layer_fusion": True,
                },
            }
        },
    ),
]

if __name__ == "__main__":

    @slurm_distribute.slurm_distribute_run(os.path.dirname(__file__))
    def experiment(
        slurm_commands_vec=None, run_command_get_stdout=run_command_get_stdout
    ):

        for name, primal_dual_config in evaluation_vec:
            for d in d_vec:
                filename = os.path.join(script_dir, f"{name}_d{d}.txt")
                results = []

                for idx, p in enumerate(p_vec):

                    syndrome_file_path = os.path.join(
                        tmp_dir, f"generated-d{d}-p{p}.syndromes"
                    )
                    benchmark_profile_path = os.path.join(
                        tmp_dir, f"{name}_d{d}-p{p}.profile"
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
                        command += ["--primal-dual-type", "embedded-comb"]
                        command += [
                            "--primal-dual-config",
                            json.dumps(primal_dual_config),
                        ]
                        command += [
                            "--benchmark-profiler-output",
                            benchmark_profile_path,
                        ]
                        if slurm_commands_vec is not None:
                            slurm_commands_vec.sanity_checked_append(command)
                            continue
                        print(" ".join(command))

                        stdout, returncode = run_command_get_stdout(command)
                        print("\n" + stdout)
                        assert returncode == 0, "command fails..."

                    if slurm_commands_vec is not None:
                        continue

                    print(benchmark_profile_path)
                    profile = Profile(
                        benchmark_profile_path,
                        apply_entries=lambda x: (
                            x["defect_num"],
                            x["solver_profile"]["primal"]["offloaded"],
                        ),
                    )
                    offloaded = 0
                    for entry in profile.entries:
                        # ["solver_profile"]["primal"]["offloaded"]
                        offloaded += entry[1]
                    defect_num = 0
                    for entry in profile.entries:
                        # ["defect_num"]
                        defect_num += entry[0]
                    del profile

                    offloading_rate = 0
                    confidence_interval = math.nan
                    if defect_num > 0:
                        offloading_rate = offloaded / defect_num
                        confidence_interval = (
                            math.sqrt(
                                1.96
                                * (
                                    offloading_rate
                                    * (1.0 - offloading_rate)
                                    / defect_num
                                )
                            )
                            / offloading_rate
                        )
                    print(
                        f"d{d}, p {p}: defect_num {defect_num}, offloaded {offloaded}, offloading_rate: {offloading_rate}, confidence: {confidence_interval}"
                    )
                    print_result = "%f %d %d %f %.2e" % (
                        p,
                        defect_num,
                        offloaded,
                        offloading_rate,
                        confidence_interval,
                    )
                    results.append(print_result)

                if slurm_commands_vec is None:
                    print("\n\n")
                    print("\n".join(results))
                    print("\n\n")

                    with open(filename, "w", encoding="utf8") as f:
                        f.write("# <p> <total_defects> <offloaded> <offloading_rate>\n")
                        f.write("\n".join(results) + "\n")
