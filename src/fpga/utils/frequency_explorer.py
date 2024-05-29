from typing import Callable, Optional
from datetime import datetime
import math, os
from dataclasses import dataclass

"""
Frequency explorer of arbitrary design
"""

BEST_FREQUENCY_KEYWORD = "[found best frequency] "
BEST_DIVIDE_BY_KEYWORD = "[found best clock_divide_by] "


def get_log_best_value(log_filepath: str, keyword: str) -> Optional[int]:
    if not os.path.exists(log_filepath):
        return None
    with open(log_filepath, "r", encoding="utf8") as f:
        for line in f.readlines():
            line = line.strip("\r\n ")
            time_example = "[2024-05-28 22:09:03] "
            if len(line) < len(time_example):
                continue
            line = line[len(time_example) :]
            if line.startswith(keyword):
                value = int(line[len(keyword) :])
                return value
    return None


def log_to_file(log_filepath: str, message: str) -> None:
    time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{time}] {message}"
    print(line)
    with open(log_filepath, "a", encoding="utf8") as f:
        f.write(line + "\n")


@dataclass
class FrequencyExplorer:
    """
    This explorer will start with `max_frequency`, build the design and see what is the maximum achievable frequency
    and try again, until a frequency is achieved.
    """

    compute_next_maximum_frequency: Callable[[int], int]
    log_filepath: str
    max_frequency: int = 300
    max_iteration: int = 5
    min_decrease: float = 0.05  # at least decrease the frequency by 10% each iteration

    def get_log_best_frequency(self) -> Optional[int]:
        return get_log_best_value(self.log_filepath, BEST_FREQUENCY_KEYWORD)

    def log(self, message):
        log_to_file(self.log_filepath, message)

    # return the maximum frequency that can be achieved; None if cannot finish within iterations
    def optimize(self) -> Optional[int]:
        log_best_frequency = self.get_log_best_frequency()
        if log_best_frequency is not None:
            # print(f"best frequency {log_best_frequency}MHz in {self.log_filepath}")
            return log_best_frequency

        frequency = int(self.max_frequency)
        self.log("optimization start")
        for iteration in range(self.max_iteration):
            self.log(f"iteration {iteration}: trying frequency {frequency}")
            new_frequency = int(self.compute_next_maximum_frequency(frequency))
            if new_frequency >= frequency:
                self.log(f"{BEST_FREQUENCY_KEYWORD}{frequency}")
                return frequency
            # if not achievable, use the new frequency
            self.log(f"suggested achievable frequency is {new_frequency}")
            if new_frequency > frequency * (1 - self.min_decrease):
                new_frequency = math.floor(frequency * (1 - self.min_decrease))
            frequency = new_frequency
        return None


@dataclass
class ClockDivideByExplorer:
    """
    This explorer will start with `min_divide_by`, and increase it until the timing constraint can be achieved.
    To use this module, it is recommended to select a frequency that is achievable for the AXI4 bus side.
    If it's the AXI4 side that is the limiting factor, then this script will fail after the `max_iteration` iterations.
    """

    compute_next_minimum_divide_by: Callable[[int], int]
    log_filepath: str
    min_divide_by: int = 2  # starting point
    frequency: int = (
        250  # an achievable frequency, see in benchmark/hardware/frequency_optimization/axi4_with_small_code
    )
    max_iteration: int = 5

    def get_log_best_divide_by(self) -> Optional[int]:
        return get_log_best_value(self.log_filepath, BEST_DIVIDE_BY_KEYWORD)

    def log(self, message):
        log_to_file(self.log_filepath, message)

    # return the minimum clock_divide_by that can be achieved; None if cannot finish within iterations
    def optimize(self) -> Optional[int]:
        log_best_divide_by = self.get_log_best_divide_by()
        if log_best_divide_by is not None:
            # print(f"best clock_divide_by {log_best_divide_by}MHz in {self.log_filepath}")
            return log_best_divide_by

        divide_by = int(self.min_divide_by)
        self.log("optimization start")
        for iteration in range(self.max_iteration):
            self.log(f"iteration {iteration}: trying divide_by {divide_by}")
            new_divide_by = int(self.compute_next_minimum_divide_by(divide_by))
            if new_divide_by <= divide_by:
                self.log(f"{BEST_DIVIDE_BY_KEYWORD}{divide_by}")
                return divide_by
            # if not achievable, use the new divide_by
            self.log(f"suggested achievable divide_by is {new_divide_by}")
            if new_divide_by > divide_by * (1 - self.min_decrease):
                new_divide_by = math.floor(divide_by * (1 - self.min_decrease))
            divide_by = new_divide_by
        return None
