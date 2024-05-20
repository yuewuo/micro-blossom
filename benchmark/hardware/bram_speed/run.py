import os
import sys
import subprocess
import math
from datetime import datetime
import re

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

interval = 10  # how many frequencies between 100MHz and 200MHz in the log scale
frequency_of = lambda idx: math.floor(100 * (2 ** (idx / interval)))
f_vec = [frequency_of(i) for i in range(interval * 5) if frequency_of(i) <= 350]
print(f_vec)


def hardware_proj_name(frequency) -> str:
    return f"f_{frequency}"


def hardware_proj_dir(frequency) -> str:
    return os.path.join(hardware_dir, hardware_proj_name(frequency))


def main():

    for frequency in f_vec:
        log_file_path = os.path.join(hardware_proj_dir(frequency), "run.log")

        if not os.path.exists(log_file_path):
            # build bram benchmark application
            make_env = os.environ.copy()
            make_env["EMBEDDED_BLOSSOM_MAIN"] = "test_bram"
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

            # rebuild the program, to make sure the code is up-to-date
            process = subprocess.Popen(
                ["make"],
                universal_newlines=True,
                cwd=hardware_proj_dir(frequency),
            )
            process.wait()
            assert process.returncode == 0, "synthesis error"

            # run on hardware and get output
            print(f"testing frequency={frequency}, log output to {log_file_path}")
            with open(log_file_path, "w", encoding="utf8") as log:
                log.write(
                    f"[host_event] [make run_a72 start] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
                )
                tty_output, command_output = get_ttyoutput(
                    command=["make", "run_a72"],
                    cwd=hardware_proj_dir(frequency),
                    silent=True,
                )
                log.write(
                    f"[host_event] [make run_a72 finish] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
                )
                log.write(tty_output + "\n")

    # gather the data
    data = ["# <frequency> <write> <read> <w-r> <r-w> <read 128> <read 256>"]
    for frequency in f_vec:
        log_file_path = os.path.join(hardware_proj_dir(frequency), "run.log")
        with open(log_file_path, "r", encoding="utf8") as f:
            lines = [
                line.strip("\r\n ")
                for line in f.readlines()
                if line.strip("\r\n ") != ""
            ]

            def find_test_result(name: str) -> float:
                lidx = None
                for idx, line in enumerate(lines):
                    if name in line:
                        assert lidx is None, "find duplicate entry"
                        lidx = idx
                assert lidx is not None, "cannot find match entry"
                results = lines[lidx + 2 : lidx + 5]
                values = []
                for idx in range(3):
                    line = results[idx]
                    assert line.startswith(f"[{idx+1}/3]")
                    match = re.search(r"per_op: (([0-9]*[.])?[0-9]+) ns", line)
                    value = float(match.group(1))
                    values.append(value)
                return sum(values) / 3

            write = find_test_result("2. Write Speed Test")
            read = find_test_result("3. Read Speed Test")
            write_then_read = find_test_result("4. Write-then-Read Speed Test")
            read_then_write = find_test_result("5. Read-then-Write Speed Test")
            read_128 = find_test_result("10. Batch Read Test using memcpy 128 bits")
            read_256 = find_test_result("11. Batch Read Test using memcpy 256 bits")
            data.append(
                f"{frequency:.2f} {write:.2f} {read:.2f} {write_then_read:.2f} {read_then_write:.2f} {read_128:.2f} {read_256:.2f}"
            )

    with open(os.path.join(this_dir, "data.txt"), "w", encoding="utf8") as f:
        f.write("\n".join(data))


if __name__ == "__main__":
    main()
