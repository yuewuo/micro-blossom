import os
import re
from dataclasses import dataclass
from typing import Optional
import subprocess


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


@dataclass
class NetListLogicEntry:
    used: int
    fixed: int
    prohibited: int
    available: int
    less: bool
    util_percent: float


@dataclass
class NetListLogic:
    registers: NetListLogicEntry
    register_as_flip_flop: NetListLogicEntry
    register_as_latch: NetListLogicEntry

    clb_luts: NetListLogicEntry
    lut_as_logic: NetListLogicEntry
    lut_as_memory: NetListLogicEntry


@dataclass
class ImplUtilizationReport:
    netlist_logic: NetListLogic

    @staticmethod
    def from_file(filepath: str) -> "ImplUtilizationReport":
        with open(filepath, "r", encoding="utf8") as f:
            lines = [line.strip("\r\n ") for line in f.readlines()]
            table_of_contents_lidx = lines.index("Table of Contents")
            while lines[table_of_contents_lidx] != "":
                table_of_contents_lidx += 1
            # ignore the titles in the table of contents
            lines = lines[table_of_contents_lidx:]
            netlist_logic_lidx = lines.index("1. Netlist Logic")
            clb_distribution_lidx = lines.index("2. CLB Distribution")
            netlist_logic_rpt = "\n".join(
                lines[netlist_logic_lidx:clb_distribution_lidx]
            )
        fields = [
            ("Registers", "registers"),
            ("Register as Flip Flop", "register_as_flip_flop"),
            ("Register as Latch", "register_as_latch"),
            ("CLB LUTs", "clb_luts"),
            ("LUT as Logic", "lut_as_logic"),
            ("LUT as Memory", "lut_as_memory"),
        ]
        values = {}
        for site_type, key in fields:
            match = re.search(
                r"\|\s*"
                + site_type
                + r"\s*\|\s*(\d*)\s*\|\s*(\d*)\s*\|\s*(\d*)\s*\|\s*(\d*)\s*\|\s*(<?)(([0-9]*[.])?[0-9]+)\s*\|",
                netlist_logic_rpt,
            )
            assert match is not None
            entry = NetListLogicEntry(
                used=int(match.group(1)),
                fixed=int(match.group(2)),
                prohibited=int(match.group(3)),
                available=int(match.group(4)),
                less=match.group(5) == "<",
                util_percent=float(match.group(6)),
            )
            values[key] = entry
        netlist_logic = NetListLogic(**values)
        return ImplUtilizationReport(netlist_logic=netlist_logic)


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

    def default_impl_utilization_path(self) -> str:
        return os.path.join(self.vivado_dir, f"impl_utilization_{self.name}.txt")

    def report_impl_utilization(
        self, filepath: Optional[str] = None, force_regenerate: bool = True
    ) -> ImplUtilizationReport:
        if filepath is None:
            # use the default path
            filepath = self.default_impl_utilization_path()
        if os.path.exists(filepath) and not force_regenerate:
            print(
                f"reusing impl utilization report for {self.name} (force_regenerate=False)"
            )
            return ImplUtilizationReport.from_file(filepath)
        print(f"generating impl utilization report for {self.name}")
        # first generate a tcl script
        tcl_path = os.path.join(self.vivado_dir, f"impl_utilization_{self.name}.tcl")
        log_path = os.path.join(self.vivado_dir, f"impl_utilization_{self.name}.log")
        xpr_path = os.path.join(self.vivado_dir, f"{self.name}.xpr")
        with open(tcl_path, "w", encoding="utf8") as f:
            f.write(
                f"""
open_project {xpr_path}
open_run impl_1
report_utilization -file {filepath}
"""
            )
        # run the tcl script to generate the report file
        with open(log_path, "a", encoding="utf8") as log:
            process = subprocess.Popen(
                ["vivado", "-mode", "batch", "-source", tcl_path],
                universal_newlines=True,
                stdout=log.fileno(),
                stderr=log.fileno(),
                cwd=self.project_dir,
            )
            process.wait()
            assert process.returncode == 0, "synthesis error"
        # then return the report file
        return ImplUtilizationReport.from_file(filepath)
