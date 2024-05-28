from abc import ABC, abstractmethod
from datetime import datetime

"""
Frequency explorer of arbitrary design
"""


class FrequencyExploreTarget(ABC):
    """
    build the design and return an estimated new frequency that is achievable
    """

    @abstractmethod
    def run(self, frequency: int) -> int:
        raise NotImplementedError("run() must be implemented")


class FrequencyExplorer:
    """
    This explorer will start with `max_frequency`, build the design and see what is the maximum achievable frequency
    and try again, until a frequency is achieved.
    """

    def __init__(
        self,
        target: FrequencyExploreTarget,
        log_filename: str,
        max_frequency: int = 300,
        max_try: int = 5,
    ):
        self.target = target
        self.max_frequency = max_frequency
        self.max_try = max_try
        self.log_filename = log_filename

    def log(self, message):
        time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        line = f"[{time}] {message}"
        print(line)
        with open(self.log_filename, "a", encoding="utf8") as f:
            f.write(line + "\n")

    def optimize(self):
        with open(self.log_filename, "a", encoding="utf8") as log:
            self.log("optimization start")
            for try_index in range(self.max_try):
                self.log("optimization start")
                log.write()
                ...
