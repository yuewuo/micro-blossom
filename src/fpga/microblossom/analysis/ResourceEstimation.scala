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
  val distances = Seq(3, 5, 7, 9, 11, 13)

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
      lines += ("") // add empty line to separate blocks
    }
    lines.mkString("\n")
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
          for (_ <- 0 until 2) { // because each edge has two component
            reports(key) ++
          }
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(EdgeIsTight(leftGrownBits, rightGrownBits, config.weightBits)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
          reports(key) ++
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationEdgeRemaining'
class ResourceEstimationEdgeRemaining extends AnyFunSuite {
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
          val repeatedReport =
            RepeatedReport(Config.report(EdgeRemaining(leftGrownBits, rightGrownBits, config.weightBits)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationEdgeResponse'
class ResourceEstimationEdgeResponse extends AnyFunSuite {
  test("estimation") {
    case class Key() {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (edgeIndex <- 0 until config.edgeNum) {
        val key = Key()
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(EdgeResponse(config.vertexBits, config.weightBits)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationOffloader'
class ResourceEstimationOffloader extends AnyFunSuite {
  test("estimation") {
    case class Key(offloaderType: String, numVertices: Int, numEdges: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (offloaderIndex <- 0 until config.offloaderNum) {
        val (_edgeIndex, neighborVertices, neighborEdges) = config.offloaderInformation(offloaderIndex)
        val key = Key(config.offloaderTypeOf(offloaderIndex), neighborVertices.length, neighborEdges.length)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(Offloader(config, offloaderIndex)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexOffloadStalled'
class ResourceEstimationVertexOffloadStalled extends AnyFunSuite {
  test("estimation") {
    case class Key(numEdges: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val numEdges = config.numIncidentEdgeOf(vertexIndex)
        val key = Key(numEdges)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(OffloadStalled(numEdges)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexIsUniqueTight'
class ResourceEstimationVertexIsUniqueTight extends AnyFunSuite {
  test("estimation") {
    case class Key(numEdges: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val numEdges = config.numIncidentEdgeOf(vertexIndex)
        val key = Key(numEdges)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(VertexIsUniqueTight(numEdges)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexPostExecuteState'
class ResourceEstimationVertexPostExecuteState extends AnyFunSuite {
  test("estimation") {
    case class Key(grownBits: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val grownBits = config.grownBitsOf(vertexIndex)
        val key = Key(grownBits)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(VertexPostExecuteState(config, vertexIndex)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexPostUpdateState'
class ResourceEstimationVertexPostUpdateState extends AnyFunSuite {
  test("estimation") {
    case class Key(grownBits: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val grownBits = config.grownBitsOf(vertexIndex)
        val key = Key(grownBits)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(VertexPostUpdateState(config, vertexIndex)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexPropagatingPeer'
class ResourceEstimationVertexPropagatingPeer extends AnyFunSuite {
  test("estimation") {
    case class Key(grownBits: Int, numNeighbors: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val grownBits = config.grownBitsOf(vertexIndex)
        val numNeighbors = config.numIncidentEdgeOf(vertexIndex)
        val key = Key(grownBits, numNeighbors)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(VertexPropagatingPeer(config, vertexIndex)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexResponse'
class ResourceEstimationVertexResponse extends AnyFunSuite {
  test("estimation") {
    case class Key(grownBits: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val grownBits = config.grownBitsOf(vertexIndex)
        val key = Key(grownBits)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(VertexResponse(config, vertexIndex)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}

// sbt 'testOnly microblossom.analysis.ResourceEstimationVertexShadow'
class ResourceEstimationVertexShadow extends AnyFunSuite {
  test("estimation") {
    case class Key(grownBits: Int) {}
    val aggregated = AggregatedReport[Key]()
    for (d <- Config.distances) {
      val reports = Map[Key, RepeatedReport]()
      val config = Config.dualConfig(d)
      for (vertexIndex <- 0 until config.vertexNum) {
        val grownBits = config.grownBitsOf(vertexIndex)
        val key = Key(grownBits)
        if (reports.contains(key)) {
          reports(key) ++
        } else {
          val repeatedReport =
            RepeatedReport(Config.report(VertexShadow(config, vertexIndex)))
          println(s"d = $d, $key single: ${repeatedReport.countSingleCLBLUT} LUTs")
          reports += ((key, repeatedReport))
        }
      }
      aggregated += ((s"d = $d", reports))
      println(aggregated.simpleReport())
      println(this.getClass.getSimpleName)
      println(aggregated.detailedReport())
    }
  }
}
