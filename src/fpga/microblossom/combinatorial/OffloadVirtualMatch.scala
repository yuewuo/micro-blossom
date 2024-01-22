package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

object OffloadVirtualMatch {

  /** we need the information of the neighboring vertices of regularVertex other than the virtual vertex */
  def build(
      condition: Bool, // output
      neighborVertexStalled: Seq[Bool], // output
      edgeIsTight: Bool,
      virtualIsVirtual: Bool,
      regularIsDefect: Bool,
      regularSpeed: Speed,
      neighborEdgeIsTight: Seq[Bool],
      neighborVertexIsUniqueTight: Seq[Bool],
      neighborVertexIsDefect: Seq[Bool]
  ) = {
    val numNeighbors = neighborEdgeIsTight.length
    require(neighborVertexIsDefect.length == numNeighbors)
    require(neighborVertexIsUniqueTight.length == numNeighbors)
    require(neighborVertexStalled.length == numNeighbors)

    val vertexPreConditions = Vec.fill(numNeighbors)(Bool)
    for (neighborIndex <- 0 until numNeighbors) {
      vertexPreConditions(neighborIndex) := !neighborEdgeIsTight(neighborIndex) ||
        (neighborVertexIsUniqueTight(neighborIndex) && !neighborVertexIsDefect(neighborIndex))
    }

    condition := edgeIsTight && virtualIsVirtual && regularIsDefect && (regularSpeed === Speed.Grow) && vertexPreConditions.andR

    for (neighborIndex <- 0 until numNeighbors) {
      neighborVertexStalled(neighborIndex) := condition && neighborEdgeIsTight(neighborIndex)
    }

  }
}

case class OffloadVirtualMatch(numNeighbors: Int) extends Component {
  require(numNeighbors > 0)

  val io = new Bundle {
    val edgeIsTight = in(Bool)

    val virtualIsVirtual = in(Bool)

    val regularIsDefect = in(Bool)
    val regularSpeed = in(Speed())

    val neighborEdgeIsTight = in(Vec.fill(numNeighbors)(Bool))
    val neighborVertexIsUniqueTight = in(Vec.fill(numNeighbors)(Bool))
    val neighborVertexIsDefect = in(Vec.fill(numNeighbors)(Bool))

    val condition = out(Bool)
    val neighborVertexStalled = out(Vec.fill(numNeighbors)(Bool))
  }

  OffloadVirtualMatch.build(
    io.condition,
    io.neighborVertexStalled,
    io.edgeIsTight,
    io.virtualIsVirtual,
    io.regularIsDefect,
    io.regularSpeed,
    io.neighborEdgeIsTight,
    io.neighborVertexIsUniqueTight,
    io.neighborVertexIsDefect
  )

}

// sbt 'testOnly microblossom.combinatorial.OffloadVirtualMatchTest'
class OffloadVirtualMatchTest extends AnyFunSuite {

  test("example") {
    val numNeighbors = 5
    Config.spinal().generateVerilog(OffloadVirtualMatch(numNeighbors))
  }

}

// sbt 'runMain microblossom.combinatorial.OffloadVirtualMatchEstimation'
object OffloadVirtualMatchEstimation extends App {
  val configurations = List(
    // delay: 0.37ns
    // resource: 2xLUT5, 1xLUT4
    (1, "code capacity 1 neighbors"),
    // delay: 0.38ns
    // resource: 2xLUT6, 3xLUT5, 1xLUT4
    (3, "code capacity 3 neighbors"),
    // delay: 0.52ns
    // resource: 8xLUT6, 1xLUT5, 1xLUT2
    (5, "phenomenological 5 neighbors"),
    // delay: 0.66ns (LUT6 -> LUT6)
    // resource: 12xLUT6, 1xLUT2
    (7, "circuit-level 7 neighbors")
  )
  for ((numNeighbors, name) <- configurations) {
    val reports = Vivado.report(OffloadVirtualMatch(numNeighbors))
    println(s"$name: ${reports.timing.getPathDelaysExcludingIOWorst}ns")
    reports.resource.primitivesTable.print()
  }
}
