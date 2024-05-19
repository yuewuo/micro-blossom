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
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
if True:
    from micro_util import *

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
this_dir = os.path.dirname(os.path.abspath(__file__))
hardware_dir = os.path.join(this_dir, "hardware")


@dataclass
class Configuration:
    name: str
    d_vec: list[int]
    f: float = 10.0  # 10 MHz
    p: float = 0.001
    noise_model: str = "stim-noise-model"
    max_half_weight: int = 7
    fix_noisy_measurements: Optional[int] = None
    scala_parameters: Optional[list[str]] = None

    def noisy_measurements(self, d: int) -> int:
        if self.fix_noisy_measurements is not None:
            return self.fix_noisy_measurements
        return d - 1

    def hardware_proj_name(self, d) -> str:
        return f"{self.name}_d_{d}"

    def hardware_proj_dir(self, d) -> str:
        return os.path.join(hardware_dir, self.hardware_proj_name(d))


configurations = [
    Configuration(
        name="code_capacity",
        d_vec=[3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31],
        noise_model="phenomenological",
        fix_noisy_measurements=0,
        max_half_weight=1,
    ),
    Configuration(
        name="phenomenological",
        d_vec=[3, 5, 7, 9, 11, 13, 15, 17, 19, 21],
        noise_model="phenomenological",
        max_half_weight=1,
    ),
    Configuration(name="circuit", d_vec=[3, 5, 7, 9, 11, 13, 15]),
    Configuration(
        name="circuit_offload",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        scala_parameters=["--support-offloading"],
    ),
    # Configuration(name="circuit_fusion",d_vec=[3, 5, 7, 9, 11, 13, 15]),
]


def main():
    pass


if __name__ == "__main__":
    main()
