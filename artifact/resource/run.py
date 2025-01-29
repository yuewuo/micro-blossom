import git
import sys
import os

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))

from hardware.resource_estimate.circuit_level.run import calculate

d_vec = [3, 5, 7, 9, 11, 13, 15]


def main():
    print(
        "reproducing the result of Table 4 (Resource usage and maximum clock frequency) of the Micro Blossom paper"
    )

    for d in d_vec:
        # First calculate the number of vertices, edges and the number of bits
        result = calculate(d=d, verbose=False)

        # software memory usage calculation (--features compact) (index: u16, weight: i16, layer_id: u8, timestamp: u32):
        # blossom tracker: (hit_zero_events 8B + checkpoints: 8B + grow_states: 1B) * |V| = 17B * |V|
        # layer_fusion: (vertex_layer_id 1B + pending_breaks 2B) * |V| = 3B * |V|
        # nodes: (buffer 16B + first_blossom_child 2B) = 20B * (2*|V|, because nodes can be as much as 2*|V|)
        # overall cost: (17B + 3B + 40B) = 60B * |V|
        cpu_memory = 60 * result.vertex_num

        print(result, cpu_memory)
        # exit(0)


if __name__ == "__main__":
    main()
