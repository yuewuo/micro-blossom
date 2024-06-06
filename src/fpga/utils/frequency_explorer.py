from typing import Callable, Optional
from datetime import datetime
import math, os
import traceback
from dataclasses import dataclass
from abc import ABC, abstractmethod

from vivado_builder import *

"""
Frequency explorer of arbitrary design
"""

BEST_FREQUENCY_KEYWORD = "[found best frequency] "


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

    compute_next_maximum_frequency: Callable[[int], int | None]
    log_filepath: str
    max_frequency: int = 300
    max_iteration: int = 5
    extra_decrease: float = 0.1  # set 90% of the next frequency
    on_failure_decrease: float = 0.3  # set 70% of the frequency if synthesis failed

    def get_log_best_frequency(self) -> Optional[int]:
        return get_log_best_value(self.log_filepath, BEST_FREQUENCY_KEYWORD)

    def log(self, message):
        log_to_file(self.log_filepath, message)

    # return the maximum frequency that can be achieved; None if cannot finish within iterations
    def optimize(self) -> Optional[int]:
        log_best_frequency = self.get_log_best_frequency()
        if log_best_frequency is not None:
            # print(f"best frequency {log_best_frequency}MHz in {self.log_filepath}")
            # check it is indeed achieved
            new_frequency = self.compute_next_maximum_frequency(log_best_frequency)
            if new_frequency is None:
                return log_best_frequency
            # start from this frequency
            self.max_frequency = math.floor((1 - self.extra_decrease) * new_frequency)

        frequency = int(self.max_frequency)
        self.log("optimization start")
        for iteration in range(self.max_iteration):
            self.log(f"iteration {iteration}: trying frequency {frequency}")
            try:
                new_frequency = self.compute_next_maximum_frequency(frequency)
                self.log(f"suggested achievable frequency is {new_frequency}MHz")
                if new_frequency is None:
                    self.log(f"{BEST_FREQUENCY_KEYWORD}{frequency}")
                    return frequency
                new_frequency = math.floor((1 - self.extra_decrease) * new_frequency)
            except Exception:
                print(traceback.format_exc())
                new_frequency = math.floor((1 - self.on_failure_decrease) * frequency)
                self.log(f"synthesis failed, try next frequency {new_frequency}MHz")
            frequency = new_frequency
        return None


class OptimizableConfiguration(ABC):
    @abstractmethod
    def frequency_log_dir(self) -> str: ...

    @abstractmethod
    def init_frequency(self) -> int: ...

    @abstractmethod
    def get_project(self, frequency: int | None = None) -> MicroBlossomAxi4Builder: ...

    def optimized_project(self) -> MicroBlossomAxi4Builder:
        frequency_log_dir = self.frequency_log_dir()
        if not os.path.exists(frequency_log_dir):
            os.mkdir(frequency_log_dir)

        def compute_next_maximum_frequency(frequency: int) -> int | None:
            project = self.get_project(frequency=frequency)
            project.build()
            return project.next_maximum_frequency()

        explorer = FrequencyExplorer(
            compute_next_maximum_frequency=compute_next_maximum_frequency,
            log_filepath=os.path.join(frequency_log_dir, self.name() + ".txt"),
            max_frequency=self.init_frequency(),
        )

        best_frequency = explorer.optimize()
        return self.get_project(frequency=best_frequency)
