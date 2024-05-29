from typing import Callable, Optional
from datetime import datetime
import math, os
from dataclasses import dataclass

"""
Frequency explorer of arbitrary design
"""

BEST_FREQUENCY_KEYWORD = "[found best frequency] "


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
        if not os.path.exists(self.log_filepath):
            return None
        with open(self.log_filepath, "r", encoding="utf8") as f:
            for line in f.readlines():
                line = line.strip("\r\n ")
                time_example = "[2024-05-28 22:09:03] "
                if len(line) < len(time_example):
                    continue
                line = line[len(time_example) :]
                if line.startswith(BEST_FREQUENCY_KEYWORD):
                    frequency = float(line[len(BEST_FREQUENCY_KEYWORD) :])
                    return frequency
        return None

    def log(self, message):
        time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        line = f"[{time}] {message}"
        print(line)
        with open(self.log_filepath, "a", encoding="utf8") as f:
            f.write(line + "\n")

    # return the maximum frequency that can be achieved; None if cannot finish within iterations
    def optimize(self) -> Optional[int]:
        log_best_frequency = self.get_log_best_frequency()
        if log_best_frequency is not None:
            print(
                f"best frequency {log_best_frequency}MHz found in log file {self.log_filepath}"
            )
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
