package microblossom.util

import spinal.core._
import spinal.lib._
import microblossom._
import sys.process._
import scala.io.Source
import scala.collection.mutable
import scala.util.Try
import scala.util.Random
import scala.util.matching.Regex
import java.nio.file.{Paths, Files}
import java.nio.charset.StandardCharsets

object VivadoTarget {

  // val partName = "xc7z010clg400-1"
  // val partName = "xc7z045ffg900-2" // Zynq UltraScale+ ZCU106 Evaluation Platform (license required)
  val partName = "xcvm1802-vsva2197-2MP-e-S" // Versal VMK180 Evaluation Platform (license required)

}

case class VivadoReports(targetDirectory: String) {
  val timing = TimingReport(s"$targetDirectory/timing.txt")
  val resource = ResourceReport(s"$targetDirectory/resource.txt")
}

object Vivado {

  /** when `useImpl` is true, run implementation to get a better estimation of timing */
  def report[T <: Component](
      component: => T,
      useImpl: Boolean = false,
      removeVivadoProj: Boolean = false,
      numJobs: Option[Int] = None // default to the number of cores - 1
  ): VivadoReports = {

    val projectName = Random.alphanumeric.filter(_.isLetter).take(10).mkString
    val targetDirectory = s"gen/tmp/$projectName"
    println(s"targetDirectory: $targetDirectory")
    val spinalReport = Config.spinal(targetDirectory).generateVerilog(component)
    // find out the number of cores
    val numCores = Runtime.getRuntime().availableProcessors()
    val numJobsVal = numJobs.getOrElse((numCores - 1).min(1))
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
    val propertySettingsArray = mutable.ArrayBuffer[String]()
    for (port <- component.getAllIo) {
      val suffix = if (port.isInstanceOf[Bool]) {
        ""
      } else {
        "[*]"
      }
      val signalName = port.getName + suffix
      propertySettingsArray.append(s"""
if { [llength [get_nets $signalName]] != 0 } {
  set_property DONT_TOUCH true [get_nets $signalName]
}""")
    }
    val propertySettings = propertySettingsArray.mkString("\n")
    val runJobScript = if (useImpl) {
      s"""
launch_runs synth_1 -jobs $numJobsVal
wait_on_run synth_1
launch_runs impl_1 -jobs $numJobsVal
wait_on_run impl_1
open_run impl_1
      """
    } else {
      s"""
launch_runs synth_1 -jobs $numJobsVal
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
report_utilization -file ./resource.txt
"""
    Files.write(Paths.get(s"$targetDirectory/create_project.tcl"), script.getBytes(StandardCharsets.UTF_8))
    // run the TCL script to create vivado project
    val command = "vivado -mode batch -source create_project.tcl"
    val folder = new java.io.File(targetDirectory)
    val stdoutFile = new java.io.File(s"$targetDirectory/output.txt")
    // val output = (Process(command, folder) #> stdoutFile).!!
    val output = Process(command, folder).!!
    // remove temporary vivado project when the command succeeded
    if (removeVivadoProj) {
      Process(s"rm -rf $projectName", folder).!!
    }

    VivadoReports(targetDirectory)

  }

}
