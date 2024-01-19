package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite
import microblossom.types._

object VertexResponse {
  def build(
      maxGrowable: ConvergecastMaxGrowable, // output
      state: VertexState
  ) = {

    maxGrowable.length := maxGrowable.length.maxValue

    when(state.speed === Speed.Shrink) {
      if (state.grown.getWidth > maxGrowable.length.getWidth) {
        when(!state.grown(state.grown.high downto maxGrowable.length.getWidth).orR) {
          maxGrowable.length := state.grown.resized
        }
      } else {
        maxGrowable.length := state.grown.resized
      }
    }

  }
}

case class VertexResponse(config: DualConfig, vertexIndex: Int) extends Component {

  val io = new Bundle {
    val state = in(VertexState(config.vertexBits, config.grownBitsOf(vertexIndex)))

    val maxGrowable = out(ConvergecastMaxGrowable(config.weightBits))
  }

  VertexResponse.build(io.maxGrowable, io.state)

}

// sbt 'testOnly microblossom.combinatorial.VertexResponseTest'
class VertexResponseTest extends AnyFunSuite {

  test("example") {
    val config = DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json")
    val vertexIndex = 3
    Config.spinal().generateVerilog(VertexResponse(config, vertexIndex))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexResponseEstimation'
class VertexResponseEstimation extends AnyFunSuite {

  test("logic delay") {
    def dualConfig(name: String): DualConfig = {
      DualConfig(filename = s"./resources/graphs/example_$name.json"),
    }
    val configurations = List(
      // delay: 0.05ns
      // resource: 2xLUT4
      (dualConfig("code_capacity_d5"), 1, "code capacity 2 neighbors"),
      // delay: 0.05ns
      // resource: 2xLUT4
      (dualConfig("code_capacity_rotated_d5"), 10, "code capacity 4 neighbors"),
      // delay: 0.05ns
      // resource: 2xLUT4
      (dualConfig("phenomenological_rotated_d5"), 64, "phenomenological 6 neighbors"),
      // delay: 0.05ns
      // resource: 4xLUT4
      (dualConfig("circuit_level_d5"), 63, "circuit-level 12 neighbors")
    )
    for ((config, vertexIndex, name) <- configurations) {
      val reports = Vivado.report(VertexResponse(config, vertexIndex))
      println(s"$name: ${reports.timing.getPathDelaysExcludingIOWorst}ns")
      reports.resource.primitivesTable.print()
    }
  }

}
