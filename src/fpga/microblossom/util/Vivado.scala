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
  val partName = "xc7z045ffg900-2" // Zynq UltraScale+ ZCU106 Evaluation Platform (license required)
  // val partName = "xcvm1802-vsva2197-2MP-e-S" // Versal VMK180 Evaluation Platform (license required)

}

case class TimingReportPathNode(
    var increment: Double,
    var accumulate: Double,
    var location: String = null,
    var delayName: String = null,
    var delayAux: String = null,
    var netlistResource: String = null
)

case class TimingReportPath(
    var slack: String = null,
    var source: String = null,
    var destination: String = null,
    var dataPathDelay: String = null,
    var logicLevels: String = null,
    var details: mutable.ArrayBuffer[TimingReportPathNode] = null
)

case class TimingReport(filepath: String) {
  val paths = mutable.ArrayBuffer[TimingReportPath]()

  val source = Source.fromFile(filepath)
  try {
    var path: TimingReportPath = null
    var capturePathNode = false
    val pathNodeRegex: Regex = """(\S+) \(([^)]*)\)\s+(\d+.\d+)\s+(\d+.\d+)\s+\w?\s+(\S+)""".r
    for (originalLine <- source.getLines()) {
      val line = originalLine.trim
      if (line.startsWith("Slack:")) {
        path = TimingReportPath()
        path.slack = line.split(":")(1).trim
      } else if (line.startsWith("Source:")) {
        path.source = line.split(":")(1).trim
      } else if (line.startsWith("Destination:")) {
        path.destination = line.split(":")(1).trim
      } else if (line.startsWith("Data Path Delay:")) {
        path.dataPathDelay = line.split(":")(1).trim
      } else if (line.startsWith("Logic Levels:")) {
        path.logicLevels = line.split(":")(1).trim
      } else if (path != null && line.startsWith("-------------------------")) {
        if (path.details == null) {
          path.details = mutable.ArrayBuffer()
          capturePathNode = true
        } else {
          paths.append(path)
          path = null
          capturePathNode = false
        }
      } else {
        if (capturePathNode) {
          line match {
            case pathNodeRegex(delayName, delayAux, increment, accumulate, netlistResource) =>
              path.details.append(
                TimingReportPathNode(
                  delayName = delayName,
                  delayAux = delayAux,
                  increment = increment.toDouble,
                  accumulate = accumulate.toDouble,
                  netlistResource = netlistResource
                )
              )
            case _ => // no nothing
          }
        }
      }
    }
  } finally {
    source.close()
  }

  /** analyze delay excluding IBUF and OBUF and their associated net delay */
  def getPathDelaysExcludingIO(): List[Double] = {
    val delays = mutable.ArrayBuffer[Double]()
    for (path <- paths) {
      var startingTime = path.details(0).accumulate
      var endingTime = path.details(path.details.length - 1).accumulate
      for ((pathNode, index) <- path.details.zipWithIndex) {
        if (pathNode.delayName == "IBUF") {
          startingTime = path.details(index + 1).accumulate
        }
        if (pathNode.delayName == "OBUF") {
          endingTime = path.details(index - 2).accumulate
        }
      }
      delays.append(endingTime - startingTime)
    }
    delays.toList
  }

  def getPathDelaysExcludingIOWorst(): Double = getPathDelaysExcludingIO.max

}

object Vivado {

  /** when `useImpl` is true, run implementation to get a better estimation of timing */
  def reportTiming[T <: Component](component: => T, useImpl: Boolean = false): TimingReport = {

    // return TimingReport("gen/tmp/gwlkxvkqjF/timing.txt")

    val projectName = Random.alphanumeric.filter(_.isLetter).take(10).mkString
    val targetDirectory = s"gen/tmp/$projectName"
    println(s"targetDirectory: $targetDirectory")
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
    // val output = (Process(command, folder) #> stdoutFile).!!
    val output = Process(command, folder).!!
    // remove temporary vivado project when the command succeeded
    Process(s"rm -rf $projectName", folder).!!
    TimingReport(s"$targetDirectory/timing.txt")
  }
}
