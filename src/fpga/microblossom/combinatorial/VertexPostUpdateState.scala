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

// sbt 'runMain microblossom.combinatorial.VertexPostUpdateStateEstimation'
object VertexPostUpdateStateEstimation extends App {
  def dualConfig(name: String): DualConfig = {
    DualConfig(filename = s"./resources/graphs/example_$name.json"),
  }
  val configurations = List(
    // delay: 0.41ns
    // resource: 1xLUT6, 1xLUT5, 10xLUT4, 1xLUT3 -> 13
    (dualConfig("code_capacity_d5"), 1, "code capacity 2 neighbors"),
    // delay: 0.41ns
    // resource: 1xLUT6, 1xLUT5, 12xLUT4, 1xLUT3 -> 15
    (dualConfig("code_capacity_rotated_d5"), 10, "code capacity 4 neighbors"),
    // delay: 0.42ns
    // resource: 1xLUT6, 1xLUT5, 16xLUT4, 1xLUT3 -> 19
    (dualConfig("phenomenological_rotated_d5"), 64, "phenomenological 6 neighbors"),
    // delay: 0.42ns
    // resource: 1xLUT6, 18xLUT5, 1xLUT4, 1xLUT3 -> 21
    (dualConfig("circuit_level_d5"), 63, "circuit-level 12 neighbors")
  )
  for ((config, vertexIndex, name) <- configurations) {
    val reports = Vivado.report(VertexPostUpdateState(config, vertexIndex))
    println(s"$name: ${reports.timing.getPathDelaysExcludingIOWorst}ns")
    reports.resource.primitivesTable.print()
  }
}
