package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite
import microblossom.types._

object VertexPostUpdateState {
  def build(
      after: VertexState, // output
      before: VertexState,
      propagator: VertexPropagatingPeerResult,
      config: DualConfig
  ) = {

    after := before

    when(!before.isDefect && !before.isVirtual && (before.grown === 0)) {
      when(propagator.valid) {
        after.node := propagator.node
        after.root := propagator.root
        after.speed := Speed.Grow
      } otherwise {
        after.node := config.IndexNone
        after.root := config.IndexNone
        after.speed := Speed.Stay
      }
    }

  }
}

case class VertexPostUpdateState(config: DualConfig, vertexIndex: Int) extends Component {
  val grownBits = config.grownBitsOf(vertexIndex)

  val io = new Bundle {
    val before = in(VertexState(config.vertexBits, grownBits))
    val propagator = in(VertexPropagatingPeerResult(config.vertexBits))

    val after = out(VertexState(config.vertexBits, grownBits))
  }

  VertexPostUpdateState.build(
    io.after,
    io.before,
    io.propagator,
    config
  )
}

// sbt 'testOnly microblossom.combinatorial.VertexPostUpdateStateTest'
class VertexPostUpdateStateTest extends AnyFunSuite {

  test("example") {
    val config = DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json")
    val vertexIndex = 3
    Config.spinal().generateVerilog(VertexPostUpdateState(config, vertexIndex))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexPostUpdateStateDelayEstimation'
class VertexPostUpdateStateDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_d5.json"),
        1,
        "code capacity 2 neighbors"
      ), // 0.41ns
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_rotated_d5.json"),
        10,
        "code capacity 4 neighbors"
      ), // 0.41ns
      (
        DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json"),
        64,
        "phenomenological 6 neighbors"
      ), // 0.42ns
      (
        DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json"),
        63,
        "circuit-level 12 neighbors"
      ) // 0.42ns
    )
    for ((config, vertexIndex, name) <- configurations) {
      val timingReport = Vivado.reportTiming(VertexPostUpdateState(config, vertexIndex))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
