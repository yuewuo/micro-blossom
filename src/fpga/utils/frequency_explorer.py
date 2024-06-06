from typing import Callable, Optional
from datetime import datetime
import math, os
import traceback
from dataclasses import dataclass

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
