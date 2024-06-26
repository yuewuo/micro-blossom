import re, shutil, time, math
from dataclasses import dataclass, field
from typing import Protocol
from vivado_builder import *
from defects_generator import *
import numpy as np


@dataclass
class TimeDistribution:
    lower: float = 1e-9
    upper: float = 1.0
    N: int = 2000
    counter: dict[int, int] = field(default_factory=lambda: {})
    underflow_count: int = 0
    overflow_count: int = 0

    @staticmethod
    def from_line(line: str) -> "TimeDistribution":
        # example: "<lower>1.000e-9<upper>1.000e0<N>2000[666]1[695]23[696]80[698]7[699]3[underflow]0[overflow]0"
        match = re.search(
            "<lower>([\+-e\d\.]+)<upper>([\+-e\d\.]+)<N>(\d+)((?:\[\d+\]\d+)*)\[underflow\](\d+)\[overflow\](\d+)",
            line,
        )
        lower = float(match.group(1))
        upper = float(match.group(2))
        N = int(match.group(3))
        counter = {}
        if match.group(4) != "":
            for ele in match.group(4)[1:].split("["):
                index, count = ele.split("]")
                counter[int(index)] = int(count)
        underflow_count = int(match.group(5))
        overflow_count = int(match.group(6))
        return TimeDistribution(
            lower=lower,
            upper=upper,
            N=N,
            counter=counter,
            underflow_count=underflow_count,
            overflow_count=overflow_count,
        )

    # add profile from `Profile`
    def add_profile(self, profile):
        for entry in profile.entries:
            decoding_time = entry["events"]["decoded"]
            self.record(decoding_time)

    # the ratio between two latencies of neighboring bins
    @property
    def interval_ratio(self) -> float:
        return np.expm1(math.log(self.upper / self.lower) / self.N)

    def record(self, latency: float, count: int):
        if latency < self.lower:
            self.underflow_count += count
        elif latency >= self.upper:
            self.overflow_count += count
        else:
            ratio = math.log(latency / self.lower) / math.log(self.upper / self.lower)
            index = math.floor(self.N * ratio)
            assert index < self.N
            if index in self.counter:
                self.counter[index] += count
            else:
                self.counter[index] = count

    def flatten(self) -> tuple[list[float], list[int]]:
        latencies = [
            self.lower * ((self.upper / self.lower) ** (i / self.N))
            for i in range(self.N)
        ]
        counters = [self.counter.get(i) or 0 for i in range(self.N)]
        counters[0] += self.underflow_count
        counters[1] += self.overflow_count
        return latencies, counters

    def to_line(self) -> str:
        line = f"<lower>{self.lower:.3e}<upper>{self.upper:.3e}<N>{self.N}"
        for index in sorted(self.counter.keys()):
            line += f"[{index}]{self.counter[index]}"
        line += f"[underflow]{self.underflow_count}[overflow]{self.overflow_count}"
        return line

    def assert_compatible_with(self, other: "TimeDistribution"):
        assert self.lower == other.lower
        assert self.upper == other.upper
        assert self.N == other.N

    def __add__(self, other: "TimeDistribution") -> "TimeDistribution":
        self.assert_compatible_with(other)
        result = TimeDistribution(**self.__dict__)
        result.underflow_count += other.underflow_count
        result.overflow_count += other.overflow_count
        for index in other.counter.keys():
            if index in result.counter:
                result.counter[index] += other.counter[index]
            else:
                result.counter[index] = other.counter[index]
        return result

    def latency_of(self, index: int) -> float:
        return self.lower * ((self.upper / self.lower) ** ((index + 0.5) / self.N))

    def count_records(self) -> int:
        return sum(self.counter.values())

    def average_latency(self) -> float:
        sum_latency = 0
        for index in self.counter.keys():
            sum_latency += self.counter[index] * self.latency_of(index)
        return sum_latency / self.count_records()

    def bias_latency(self, additional_latency: float) -> "TimeDistribution":
        distribution = TimeDistribution(lower=self.lower, upper=self.upper, N=self.N)
        for latency, count in zip(*self.flatten()):
            latency += additional_latency
            distribution.record(latency, count)
        return distribution

    def filter_latency_range(
        self, min_latency: float, max_latency: float, assert_count: int = 1
    ) -> "TimeDistribution":
        x_vec = []
        y_vec = []
        for latency, count in zip(*self.flatten()):
            if latency < min_latency or latency > max_latency:
                assert (
                    count <= assert_count
                ), f"[warning] latency {latency} has count {count} > {assert_count}"
                continue
            x_vec.append(latency)
            y_vec.append(count)
        distribution = TimeDistribution(
            lower=min(x_vec), upper=max(x_vec), N=len(x_vec)
        )
        for x, y in zip(x_vec, y_vec):
            distribution.record(x, y)
        return distribution

    # smooth the distribution by combing adjacent bins
    def combine_bins(self, combine_bin: int = 1) -> "TimeDistribution":
        x_vec, y_vec = self.flatten()
        cx_vec = []
        cy_vec = []
        if len(x_vec) % combine_bin != 0:
            # append 0 data
            padding = math.ceil(len(x_vec) / combine_bin) - len(x_vec)
            for i in range(padding):
                x = x_vec[-1] * (self.interval_ratio ** (1 + i))
                x_vec.append(x)
                y_vec.append(0)
        for idx in range(len(x_vec) // combine_bin):
            start = idx * combine_bin
            end = (idx + 1) * combine_bin
            x = sum(x_vec[start:end]) / combine_bin
            y = sum(y_vec[start:end])
            cx_vec.append(x)
            cy_vec.append(y)
        distribution = TimeDistribution(
            lower=min(cx_vec), upper=max(cx_vec), N=len(cx_vec)
        )
        for x, y in zip(cx_vec, cy_vec):
            distribution.record(x, y)
        return distribution

    def fit_exponential_tail(
        self, f_range: tuple[float, float] | None = None
    ) -> tuple[float, float]:
        counts_records = self.count_records()
        if f_range is None:
            f_range = (10 / counts_records, 1e5 / counts_records)
        min_f, max_f = f_range
        i_vec = []
        latencies, counters = self.flatten()
        # search from large latency to small latency
        for i, counter in reversed(list(enumerate(counters))):
            if counter / counts_records < min_f:
                continue
            if counter / counts_records >= max_f:
                break
            i_vec.append(i)
        fit_latency = np.array([latencies[i] for i in i_vec])
        fit_freq = np.array([counters[i] for i in i_vec]) / counts_records

        # assume freq / (latency * interval_ratio) = exp(A - B * latency)
        fit_y = np.log(fit_freq) - np.log(fit_latency) - np.log(self.interval_ratio)

        B, A = np.polyfit(-fit_latency, fit_y, 1)
        # print(f"P(L) = exp({A} - {B} * latency)")
        return A, B

    # find a latency where accumulated probability beyond this latency is higher than certain value
    def find_cut_off_latency(self, probability: float) -> float:
        cut_off_count = self.count_records() * probability
        assert cut_off_count >= 10, "otherwise not accurate enough"
        # accumulate from right most
        x_vec, y_vec = self.flatten()
        accumulated = 0
        for idx in reversed(range(0, len(x_vec))):
            accumulated += y_vec[idx]
            if accumulated >= cut_off_count:
                return x_vec[idx + 1]


@dataclass
class BenchmarkDecodingResult:
    """
    The decoding benchmark returns two duration distributions:
    one for latency measured in hardware, the other for cpu wall time measured in software
    """

    latency: TimeDistribution
    cpu_wall: TimeDistribution

    @staticmethod
    def from_tty_output(tty_output: str) -> "BenchmarkDecodingResult":
        latency = None
        cpu_wall = None
        lines = tty_output.split("\n")
        for line in lines:
            line = line.strip("\r\n ")
            if line.startswith("latency_benchmarker<lower>"):
                latency = TimeDistribution.from_line(line)
            if line.startswith("cpu_wall_benchmarker<lower>"):
                cpu_wall = TimeDistribution.from_line(line)
        assert latency is not None
        assert cpu_wall is not None
        return BenchmarkDecodingResult(latency=latency, cpu_wall=cpu_wall)

    def __add__(self, other: "BenchmarkDecodingResult") -> "BenchmarkDecodingResult":
        return BenchmarkDecodingResult(
            latency=self.latency + other.latency,
            cpu_wall=self.cpu_wall + other.cpu_wall,
        )


class Configuration(Protocol):
    def get_graph_builder(self) -> MicroBlossomGraphBuilder: ...
    def optimized_project(self) -> MicroBlossomAxi4Builder: ...


@dataclass
class DecodingSpeedBenchmarkerBasic:
    this_dir: str
    configuration: Configuration
    p: float
    name_suffix: str = ""
    samples: int = 100
    # using either stream (layer fusion) or batch decoding
    use_layer_fusion: bool = False
    measurement_cycle_ns: int = 1000
    multiple_fusion: bool = True
    enable_detailed_print: bool = False

    def get_graph_builder(self) -> MicroBlossomGraphBuilder:
        graph_builder = self.configuration.get_graph_builder()
        graph_builder.test_syndrome_count = self.samples
        graph_builder.graph_folder = os.path.join(self.this_dir, "tmp-syndrome")
        graph_builder.name += self.name_suffix + f"_p_{self.p:.4e}_N_{self.samples}"
        graph_builder.p = self.p
        return graph_builder

    def tty_result_path(self) -> str:
        graph_builder = self.get_graph_builder()
        tty_result_path = os.path.join(self.this_dir, "tmp-tty")
        if not os.path.exists(tty_result_path):
            os.mkdir(tty_result_path)
        return os.path.join(
            tty_result_path, f"{graph_builder.name + self.name_suffix}.txt"
        )

    def generate_defects(self) -> str:
        graph_builder = self.get_graph_builder()
        print(graph_builder)
        # first check whether the file already exists
        defects_generator = LargeDefectsGenerator(graph_builder)
        return defects_generator.generate()

    # PLM Boot Time takes very long: may take 1 hour, just wait for it.
    def run(self, timeout: int = 3600, silent: bool = False) -> BenchmarkDecodingResult:
        # if result is already there, do not need to run again
        if os.path.exists(self.tty_result_path()):
            print(f"reuse existing {self.tty_result_path()}")
            with open(self.tty_result_path(), "r", encoding="utf8") as f:
                return BenchmarkDecodingResult.from_tty_output(f.read())
        # copy the defects to the folder
        defects_file_path = self.generate_defects()
        dest_file_path = os.path.join(embedded_dir, "embedded.defects")
        dest_file_ori_path = os.path.join(embedded_dir, "original.defects")
        shutil.move(dest_file_path, dest_file_ori_path)
        try:
            shutil.copyfile(defects_file_path, dest_file_path)
            # build the project
            project = self.configuration.optimized_project()
            project.create_vivado_project(update=True)  # update c files
            make_env = os.environ.copy()
            assert "USE_LAYER_FUSION" not in make_env
            if self.use_layer_fusion:
                make_env["USE_LAYER_FUSION"] = "1"
            make_env["MEASUREMENT_CYCLE_NS"] = f"{self.measurement_cycle_ns}"
            graph = SingleGraph.from_file(project.graph_builder.graph_file_path())
            make_env["NUM_LAYER_FUSION"] = f"{graph.layer_fusion.num_layers}"
            assert "DISABLE_MULTIPLE_FUSION" not in make_env
            if not self.multiple_fusion:
                make_env["DISABLE_MULTIPLE_FUSION"] = "1"
            if not self.enable_detailed_print:
                make_env["DISABLE_DETAIL_PRINT"] = "1"
            make_env["EMBEDDED_BLOSSOM_MAIN"] = "benchmark_decoding"
            project.build_embedded_binary(make_env)
            project.build_vivado_project(force_recompile_binary=True)
            assert not project.timing_sanity_check_failed()
            print("running application")
            start = time.time()
            tty_output = project.run_application(timeout=timeout, silent=silent)
            print(f"running application takes {time.time() - start}s")
            with open(self.tty_result_path(), "w", encoding="utf8") as f:
                f.write(tty_output)
        finally:
            # delete the defect file, because otherwise it might introduce confusion
            shutil.move(dest_file_ori_path, dest_file_path)
        return BenchmarkDecodingResult.from_tty_output(tty_output)


# at most run 1e6 syndromes in some batch, so that a single file is not too large
DECODING_SPEED_BENCHMARK_SINGLE_RUN_MAX_N: int = 1_000_000


class DecodingSpeedBenchmarker(DecodingSpeedBenchmarkerBasic):
    """
    a wrapper that provides chunk decoding process: split a huge evaluation into small pieces
    """

    def run(self, silent: bool = False) -> BenchmarkDecodingResult:
        if self.samples <= DECODING_SPEED_BENCHMARK_SINGLE_RUN_MAX_N:
            return super().run(silent=silent)
        else:
            chunks = math.ceil(self.samples / DECODING_SPEED_BENCHMARK_SINGLE_RUN_MAX_N)
            sum_result = None
            for i in range(chunks):
                benchmarker = DecodingSpeedBenchmarkerBasic(**self.__dict__)
                benchmarker.samples = DECODING_SPEED_BENCHMARK_SINGLE_RUN_MAX_N
                benchmarker.name_suffix += f"_chunk_{i}"
                result = benchmarker.run(silent=silent)
                # remove syndrome data because it's too large
                benchmarker.get_graph_builder().clear(clear_defect=True)
                if sum_result is None:
                    sum_result = result
                else:
                    sum_result = sum_result + result
            assert sum_result is not None
            return sum_result
