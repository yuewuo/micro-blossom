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
d_vec = [9]
# use the same physical error rate to construct the decoder for consistence with the hardware evaluation
p_graph = 0.001


def total_rounds(d, p):
    return int(100000 * ((7 / d) ** 3) * (0.001 / p))


evaluation_vec = [
    (
        "no_offloading",
        {
            "dual": {
                "log_instructions": True,
                "log_responses": True,
            }
        },
    ),
    (
        "pre_match",
        {
            "dual": {
                "log_instructions": True,
                "log_responses": True,
                "sim_config": {"support_offloading": True},
            }
        },
    ),
    (
        "layer_fusion",
        {
            "dual": {
                "log_instructions": True,
                "log_responses": True,
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

                    INSTRUCTION_TYPE_BITS = (
                        2  # enough to distinguish between 3 different instructions
                    )
                    INDEX_BITS = 10  # 450 vertices, 900 blossoms, 10 bits are enough
                    SPEED_BITS = 2  # { +1, -1, 0 }
                    RETURN_TYPE_BITS = 1  # either conflict or has a maximum growth

                    def apply_entries(entry):
                        history = entry["solver_profile"]["dual"]["history"]
                        conflicts = entry["solver_profile"]["dual"]["conflicts"]
                        # adding defects are parallel in hardware level: a single instruction
                        set_speed_count = 0
                        set_blossom_count = 0
                        load_defects_external_count = 0
                        out_bits = (
                            1  # at least we need to output the 1-bit parity in the end
                        )
                        for instruction in history:
                            if "AddDefectVertex" in instruction:
                                continue  # load defects does not require CPU in a real system
                            elif "FindObstacle" == instruction:
                                continue  # find obstacle is spontaneously issued in the hardware controller
                            elif "Grow" in instruction:
                                continue  # growth is spontaneously issued in the hardware controller
                            elif "SetSpeed" in instruction:
                                set_speed_count += 1
                                out_bits += (
                                    INDEX_BITS + SPEED_BITS + INSTRUCTION_TYPE_BITS
                                )
                            elif "SetBlossom" in instruction:
                                set_blossom_count += 1
                                out_bits += (
                                    INDEX_BITS + INDEX_BITS + INSTRUCTION_TYPE_BITS
                                )
                            elif "LoadDefectsExternal" in instruction:
                                load_defects_external_count += 1
                                out_bits += INSTRUCTION_TYPE_BITS  # always increment by 1, thus no argument
                            else:
                                raise Exception(f"unknown instruction: {instruction}")
                        conflict_count = 0
                        in_bits = 0
                        for conflict, max_growable in conflicts:
                            if conflict == "None":
                                if max_growable > 0:
                                    continue  # handled by the hardware controller
                                in_bits += RETURN_TYPE_BITS
                            else:
                                conflict_count += 1
                                in_bits += RETURN_TYPE_BITS + 6 * INDEX_BITS
                        return (
                            entry["defect_num"],
                            entry["solver_profile"]["primal"]["offloaded"],
                            set_speed_count,
                            set_blossom_count,
                            load_defects_external_count,
                            out_bits,
                            conflict_count,
                            in_bits,
                        )

                    profile = Profile(
                        benchmark_profile_path, apply_entries=apply_entries
                    )
                    offloaded = 0
                    defect_num = 0
                    set_speed_count = 0
                    set_blossom_count = 0
                    load_defects_external_count = 0
                    num_entries = len(profile.entries)
                    out_bits = 0
                    conflict_count = 0
                    in_bits = 0
                    for entry in profile.entries:
                        defect_num += entry[0]
                        offloaded += entry[1]
                        set_speed_count += entry[2]
                        set_blossom_count += entry[3]
                        load_defects_external_count += entry[4]
                        out_bits += entry[5]
                        conflict_count += entry[6]
                        in_bits += entry[7]

                    d = 9
                    regular_vertices = (
                        profile.partition_config.vertex_num * (d - 1) / (d + 1)
                    )
                    del profile

                    # also calculate the bandwidth requirement if we're just sending syndrome
                    naive_out_bits = d * d  # output the correction for each data qubit
                    # matches between defect vertices: for each defect vertex it needs the information of peer,
                    compressed_out_bits = 1
                    naive_in_bits = regular_vertices  # naive encoding of the syndrome
                    # compress using defect index encoding, plus one ending signal bit (at least one bit even no defect)
                    compressed_in_bits = (defect_num / num_entries) * (
                        INDEX_BITS
                        - 1  # because we don't need to send indices of blossoms in the syndrome
                    ) + 1

                    print_result = (
                        "%e %d %.3e %.3e %.3e %.3e %.3e %.3e %.3e %.3e %d %d %.3e %.3e"
                        % (
                            p,
                            num_entries,
                            defect_num / num_entries,
                            offloaded / num_entries,
                            set_speed_count / num_entries,
                            set_blossom_count / num_entries,
                            load_defects_external_count / num_entries,
                            out_bits / num_entries,
                            conflict_count / num_entries,
                            in_bits / num_entries,
                            naive_out_bits,
                            naive_in_bits,
                            compressed_in_bits,
                            compressed_out_bits,
                        )
                    )
                    print(print_result)
                    results.append(print_result)

                if slurm_commands_vec is None:
                    print("\n\n")
                    print("\n".join(results))
                    print("\n\n")

                    with open(filename, "w", encoding="utf8") as f:
                        f.write(
                            "# <p> len(entries) <average defect num> <average offload num> <average set speed> <average set blossom> <average load external> <avr out bits> <avr conflicts> <avr in bits> <avr naive out bits> <avr naive in bits> <avr compressed in bits>\n"
                        )
                        f.write("\n".join(results) + "\n")
