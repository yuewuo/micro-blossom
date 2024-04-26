import os
import re
from dataclasses import dataclass


@dataclass
class NetListLogic: ...


@dataclass
class UtilizationReport:
    netlist_logic: NetListLogic


def read_utilization_report(project_dir): ...


# d_9_wrapper_utilization_placed.rpt


@dataclass
class RoutedTimingSummary:
    clk_pl_0_wns: float
    clk_pl_0_tns: float

    @staticmethod
    def from_file(filepath: str) -> "RoutedTimingSummary":
        with open(filepath, "r", encoding="utf8") as f:
            rpt = f.read()
        match = re.search(r"Design Timing Summary[\s\|-]*WNS\(ns\).*\n.*\n *(.*)", rpt)
        assert match is not None
        match2 = re.search(
            r"([-+]?([0-9]*[.])?[0-9]+) *([-+]?([0-9]*[.])?[0-9]+)", match.group(1)
        )
        assert match2 is not None
        clk_pl_0_wns = float(match2.group(1))
        clk_pl_0_tns = float(match2.group(3))
        return RoutedTimingSummary(clk_pl_0_wns, clk_pl_0_tns)


class VivadoProject:
    def __init__(self, project_dir: str) -> None:
        self.name = os.path.basename(os.path.normpath(project_dir))
        self.project_dir = project_dir
        self.vivado_dir = os.path.join(project_dir, f"{self.name}_vivado")
        self.impl_dir = os.path.join(self.vivado_dir, f"{self.name}.runs", "impl_1")

    def frequency(self) -> float:
        with open(
            os.path.join(self.project_dir, "Makefile"), "r", encoding="utf8"
        ) as f:
            makefile = f.read()
        match = re.search(r"CLOCK_FREQUENCY \?= (([0-9]*[.])?[0-9]+)", makefile)
        assert match is not None
        return float(match.group(1))

    def routed_timing_summery(self) -> RoutedTimingSummary:
        report_file = os.path.join(
            self.impl_dir, f"{self.name}_wrapper_timing_summary_routed.rpt"
        )
        return RoutedTimingSummary.from_file(report_file)
