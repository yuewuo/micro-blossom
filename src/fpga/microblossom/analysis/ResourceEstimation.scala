package microblossom.analysis

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._
import microblossom.combinatorial._
import microblossom.modules._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite
import scala.collection.mutable.Map
import scala.collection.mutable.ArrayBuffer
import scala.util.matching.Regex

object Config {
  val distances = Seq(3, 5, 7, 9, 11, 13, 15)

  def dualConfig(d: Int): DualConfig = {
    val config = DualConfig(filename = s"./resources/graphs/example_circuit_level_d$d.json")
    config
  }

  def report[T <: Component](
      component: => T
  ): VivadoReports = {
    require(VivadoTarget.partName.startsWith("xcvm"), "to estimate resources, use Versal target")
    Vivado.report(component, useImpl = true)
  }

  val CLBLUTPattern: Regex = """\| CLB LUTs\s*\|\s*(\d*)\s*\|[\S\s]*""".r
}

case class RepeatedReport(var report: VivadoReports, var count: Int = 1) {
  def ++() = {
    count = count + 1
  }

  def countSingleCLBLUT(): Int = {
    for (line <- report.resource.netlistLogicTable.tableLines) {
      line match {
        case Config.CLBLUTPattern(num) =>
          return num.toInt
        case _ => // no nothing
      }
    }
    throw new Exception("cannot find CLB LUT count in the table")
  }

  def countCLBLUT(): Int = {
    countSingleCLBLUT * count
  }
}

case class AggregatedReport[Key](
    val reportsVec: ArrayBuffer[(String, Map[Key, RepeatedReport])] = ArrayBuffer[(String, Map[Key, RepeatedReport])]()
) {
  def +=(elem: (String, Map[Key, RepeatedReport])) = {
    reportsVec += elem
  }

  def simpleReport(): String = {
    val lines = ArrayBuffer[String]()
    for ((name, reports) <- reportsVec) {
      var countCLBLUT = 0
      for ((key, repeatedReport) <- reports) {
        countCLBLUT += repeatedReport.countCLBLUT()
      }
      lines += (s"$name: $countCLBLUT LUTs")
    }
    lines.mkString("\n")
  }

  def detailedReport(): String = {
    val lines = ArrayBuffer[String]()
    for ((name, reports) <- reportsVec) {
      var countCLBLUT = 0
      for ((key, repeatedReport) <- reports) {
        countCLBLUT += repeatedReport.countCLBLUT()
      }
      lines += (s"$name: $countCLBLUT LUTs")
      for ((key, repeatedReport) <- reports) {
        lines += (s"    $key: #${repeatedReport.count} x ${repeatedReport.countSingleCLBLUT()} = ${repeatedReport.countCLBLUT()} LUTs")
      }
    }
    lines.mkString("\n")
  }
}

// TODO: this is for debugging, remove later
// sbt 'testOnly microblossom.analysis.Dev'
class Dev extends AnyFunSuite {
  test("estimation") {
    val report = VivadoReports("./gen/estimation_20240104_context_depth/depth_1")
    val repeatedReport = RepeatedReport(report)
    report.resource.netlistLogicTable.print()
    println(repeatedReport.countSingleCLBLUT)
    println(repeatedReport.countCLBLUT)
    case class Key(leftGrownBits: Int, rightGrownBits: Int) {}
    val reports = Map[Key, RepeatedReport]()
    reports += ((Key(2, 3), repeatedReport))
    (reports(Key(2, 3))) ++
    val aggregated = AggregatedReport[Key]()
    aggregated += (("d = 3", reports))
    println(aggregated.simpleReport())
    println(aggregated.detailedReport())
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationEdgeIsTight'
class ResourceEstimationEdgeIsTight extends AnyFunSuite {
  test("estimation") {
    case class Key(leftGrownBits: Int, rightGrownBits: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (edgeIndex <- 0 until config.edgeNum) {
        val (leftVertex, rightVertex) = config.incidentVerticesOf(edgeIndex)
        val leftGrownBits = config.grownBitsOf(leftVertex)
        val rightGrownBits = config.grownBitsOf(rightVertex)
        val key = Key(leftGrownBits, rightGrownBits)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val report = Config.report(EdgeIsTight(leftGrownBits, rightGrownBits, config.weightBits))
          report.resource.netlistLogicTable.print()
          reports += ((key, RepeatedReport(report)))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(aggregated.detailedReport())
    }
  }
}
