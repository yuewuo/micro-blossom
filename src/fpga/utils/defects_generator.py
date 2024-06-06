import os, sys, git, math
from dataclasses import dataclass, field

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *


@dataclass
class LargeDefectsGenerator:
    graph_builder: MicroBlossomGraphBuilder
    # at most generate 1e6 samples in a single file, and combine multiple files to obtain the result
    generate_syndrome_max_N: int = 1_000_000

    def generate(self) -> str:
        graph_builder = self.graph_builder
        N = graph_builder.test_syndrome_count
        # first check whether the file already exists
        defect_file_path = graph_builder.defect_file_path()
        if os.path.exists(defect_file_path):
            graph_builder.assert_defects_file_samples(N)
            return defect_file_path
        if N <= self.generate_syndrome_max_N:
            graph_builder.build()
        else:
            chunk_graph_builder = MicroBlossomGraphBuilder(**graph_builder.__dict__)
            chunk_graph_builder.name += f"_chunk"
            chunk_graph_builder.test_syndrome_count = self.generate_syndrome_max_N
            chunks = math.ceil(N / self.generate_syndrome_max_N)
            for chunk in range(chunks):
                chunk_graph_builder.clear()  # remove existing files
                chunk_graph_builder.build()
                # append the defect data
                with open(defect_file_path, "ab") as f:
                    with open(chunk_graph_builder.defect_file_path(), "rb") as tmp_f:
                        f.write(tmp_f.read())
                chunk_graph_builder.clear()
        graph_builder.assert_defects_file_samples(N)
        return defect_file_path
