import re
from dataclasses import dataclass


@dataclass
class TimeDistribution:
    lower: float
    upper: float
    N: int
    counter: dict[int, int]
    underflow_count: int
    overflow_count: int

    @staticmethod
    def from_line(line: str) -> "TimeDistribution":
        # example: "<lower>1.000e-9<upper>1.000e0<N>2000[666]1[695]23[696]80[698]7[699]3[underflow]0[overflow]0"
        match = re.search(
            "<lower>([-e\d\.]+)<upper>([-e\d\.]+)<N>(\d+)((?:\[\d+\]\d+)*)\[underflow\](\d)+\[overflow\](\d)+",
            line,
        )
        lower = float(match.group(1))
        upper = float(match.group(2))
        N = int(match.group(3))
        counter = {}
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

    def to_line(self) -> str:
        line = f"<lower>{self.lower:.3e}<upper>{self.upper:.3e}<N>{self.N}"
        for index in sorted(self.counter.keys()):
            line += f"[{index}]{self.counter[index]}"
        line += f"[underflow]{self.underflow_count}[overflow]{self.overflow_count}"
        return line


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
        return BenchmarkDecodingResult(latency=latency, cpu_wall=cpu_wall)
