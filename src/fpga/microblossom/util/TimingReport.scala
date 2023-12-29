package microblossom.util

import microblossom._
import sys.process._
import scala.io.Source
import scala.collection.mutable
import scala.util.Try
import scala.util.Random
import scala.util.matching.Regex
import java.nio.file.{Paths, Files}
import java.nio.charset.StandardCharsets

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
