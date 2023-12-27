package microblossom.util

import spinal.core._
import spinal.lib._
import microblossom._
import sys.process._
import scala.collection.mutable
import scala.util.Random
import java.nio.file.{Paths, Files}
import java.nio.charset.StandardCharsets

object VivadoTarget {

  val partName = "xc7z010clg400-1"
//   val partName = "xc7z045ffg900-2" // Zynq UltraScale+ ZCU106 Evaluation Platform
//   val partName = "xcvm1802-vsva2197-2MP-e-S" // Versal VMK180 Evaluation Platform

}

object Vivado {

  /** when `useImpl` is true, run implementation to get a better estimation of timing */
  def reportTiming[T <: Component](component: => T, useImpl: Boolean = false): TimingReport = {
    val projectName = Random.alphanumeric.filter(_.isLetter).take(10).mkString
    val targetDirectory = s"gen/tmp/$projectName"
    val spinalReport = Config.spinal(targetDirectory).generateVerilog(component)
    // create a TCL script to generate the project
    val moduleName = spinalReport.toplevelName
    val inputPorts = mutable.Set[String]()
    val outputPorts = mutable.Set[String]()
    for (port <- component.getAllIo) {
      val name = port.getName()
      if (port.isInput) {
        inputPorts += name
      } else if (port.isOutput) {
        outputPorts += name
      } else {
        ???
      }
    }
    val propertySettings = {
      for {
        port <- component.getAllIo
      } yield s"set_property DONT_TOUCH true [get_nets ${port.getName}${
          if (port.isInstanceOf[Bool]) {
            ""
          } else {
            "[*]"
          }
        }]"
    }.mkString("\n")
    val runJobScript = if (useImpl) {
      """
launch_runs synth_1 -jobs 8
wait_on_run synth_1
launch_runs impl_1 -jobs 8
wait_on_run impl_1
open_run impl_1
      """
    } else {
      """
launch_runs synth_1 -jobs 8
wait_on_run synth_1
open_run synth_1
      """
    }
    val script = s"""
create_project $projectName ./$projectName -part ${VivadoTarget.partName}
add_files ./$moduleName.v
set_property top $moduleName [current_fileset]

synth_design -rtl -rtl_skip_mlo -name rtl_1 -mode out_of_context

$propertySettings

create_clock -name virt_clk -period 10000 -waveform {0 5000}
set_input_delay 0 -clock virt_clk [all_inputs]
set_output_delay 0 -clock virt_clk [all_outputs]

$runJobScript

report_timing -from [all_inputs] -to [all_outputs] -nworst 10 -file ./timing.txt
"""
    Files.write(Paths.get(s"$targetDirectory/reportTiming.tcl"), script.getBytes(StandardCharsets.UTF_8))
    // run the TCL script to create vivado project
    val command = "vivado -mode batch -source reportTiming.tcl"
    val folder = new java.io.File(targetDirectory)
    val stdoutFile = new java.io.File(s"$targetDirectory/output.txt")
    val output = (Process(command, folder) #> stdoutFile).!!
    // Files.write(Paths.get(s"$targetDirectory/output.txt"), output.getBytes(StandardCharsets.UTF_8))

    // TODO: remove temporary vivado project
    TimingReport("TODO")
  }
}

case class TimingReport(filepath: String) {
  // TODO: analyze delay excluding IBUF and OBUF and their associated net delay
}
