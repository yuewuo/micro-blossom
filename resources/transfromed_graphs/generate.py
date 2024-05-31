import os, sys, git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *

this_dir = os.path.dirname(os.path.abspath(__file__))


def main():

    for d in range(3, 27 + 1, 2):
        generate_one("code_capacity", d, noise_model="phenomenological")

    for d in range(3, 19 + 1, 2):
        generate_one(
            "phenomenological",
            d,
            noisy_measurements=d - 1,
            noise_model="phenomenological",
        )

    for d in range(3, 7 + 1, 2):
        generate_one("circuit_level", d, noisy_measurements=d - 1, max_half_weight=7)


def generate_one(
    name: str,
    d: int,
    p: float = 0.001,
    noisy_measurements: int = 0,
    max_half_weight: int = 1,
    noise_model: str = "stim-noise-model",
):
    graph_builder = MicroBlossomGraphBuilder(
        graph_folder=this_dir,
        name=f"{name}_d{d}",
        d=d,
        p=p,
        noisy_measurements=noisy_measurements,
        max_half_weight=max_half_weight,
        noise_model=noise_model,
    )
    graph_builder.build()


if __name__ == "__main__":
    main()
