package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite
import microblossom.types._

case class VertexPropagatingPeerResult(vertexBits: Int) extends Bundle {
  val valid = Bool
  val node = Bits(vertexBits bits)
  val root = Bits(vertexBits bits)
}

object VertexPropagatingPeer {
  def build(
      peer: VertexPropagatingPeerResult, // output
      grown: UInt,
      edgeIsTight: Seq[Bool],
      peerSpeed: Seq[Speed],
      peerNode: Seq[Bits],
      peerRoot: Seq[Bits],
      config: DualConfig
  ) = {
    val numNeighbors = edgeIsTight.length
    require(peerSpeed.length == numNeighbors)
    require(peerNode.length == numNeighbors)
    require(peerRoot.length == numNeighbors)

    val propagators = Vec.fill(numNeighbors)(VertexPropagatingPeerResult(config.vertexBits))
    for (neighborIndex <- 0 until numNeighbors) {
      propagators(neighborIndex).node := peerNode(neighborIndex)
      propagators(neighborIndex).root := peerRoot(neighborIndex)
      propagators(neighborIndex).valid := edgeIsTight(neighborIndex) && (peerSpeed(neighborIndex) === Speed.Grow)
    }
    val selectedPropagator = propagators.reduceBalancedTree((l, r) => Mux(l.valid, l, r))

    // only propagate when the grown value is 0
    peer.valid := (grown === 0) && selectedPropagator.valid
    peer.node := selectedPropagator.node
    peer.root := selectedPropagator.root
  }
}

case class VertexPropagatingPeer(config: DualConfig, vertexIndex: Int) extends Component {
  val grownBits = config.grownBitsOf(vertexIndex)
  val numNeighbors = config.numIncidentEdgeOf(vertexIndex)

  val io = new Bundle {
    val grown = in(UInt(grownBits bits))

    val edgeIsTight = in(Vec.fill(numNeighbors)(Bool))
    val peerSpeed = in(Vec.fill(numNeighbors)(Speed()))
    val peerNode = in(Vec.fill(numNeighbors)(Bits(config.vertexBits bits)))
    val peerRoot = in(Vec.fill(numNeighbors)(Bits(config.vertexBits bits)))

    val peer = out(VertexPropagatingPeerResult(config.vertexBits))
  }

  VertexPropagatingPeer.build(
    io.peer,
    io.grown,
    io.edgeIsTight,
    io.peerSpeed,
    io.peerNode,
    io.peerRoot,
    config
  )
}

// sbt 'testOnly microblossom.combinatorial.VertexPropagatingPeerTest'
class VertexPropagatingPeerTest extends AnyFunSuite {

  test("example") {
    val config = DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json")
    val vertexIndex = 3
    Config.spinal().generateVerilog(VertexPropagatingPeer(config, vertexIndex))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexPropagatingPeerDelayEstimation'
class VertexPropagatingPeerDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_d5.json"),
        1,
        "code capacity 2 neighbors"
      ), // 0.36ns
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_rotated_d5.json"),
        10,
        "code capacity 4 neighbors"
      ), // 0.54ns
      (
        DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json"),
        64,
        "phenomenological 6 neighbors"
      ), // 0.63ns
      (
        DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json"),
        63,
        "circuit-level 12 neighbors"
      ) // 0.95ns (LUT3 -> LUT5 -> LUT5)
    )
    for ((config, vertexIndex, name) <- configurations) {
      val timingReport = Vivado.reportTiming(VertexPropagatingPeer(config, vertexIndex))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
