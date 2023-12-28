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
      maxLength: ConvergecastMaxLength, // output
      state: VertexState
  ) = {

    maxLength.length := maxLength.length.maxValue

    when(state.speed === Speed.Shrink) {
      if (state.grown.getWidth > maxLength.length.getWidth) {
        when(!state.grown(state.grown.high downto maxLength.length.getWidth).orR) {
          maxLength.length := state.grown.resized
        }
      } else {
        maxLength.length := state.grown.resized
      }
    }

  }
}

case class VertexResponse(config: DualConfig, vertexIndex: Int) extends Component {

  val io = new Bundle {
    val state = in(VertexState(config.vertexBits, config.grownBitsOf(vertexIndex)))

    val maxLength = out(ConvergecastMaxLength(config.weightBits))
  }

  VertexResponse.build(io.maxLength, io.state)

}

// sbt 'testOnly microblossom.combinatorial.VertexResponseTest'
class VertexResponseTest extends AnyFunSuite {

  test("example") {
    val config = DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json")
    val vertexIndex = 3
    Config.spinal().generateVerilog(VertexResponse(config, vertexIndex))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexResponseDelayEstimation'
class VertexResponseDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_d5.json"),
        1,
        "code capacity 2 neighbors"
      ), // 0.05ns
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_rotated_d5.json"),
        10,
        "code capacity 4 neighbors"
      ), // 0.05ns
      (
        DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json"),
        64,
        "phenomenological 6 neighbors"
      ), // 0.05ns
      (
        DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json"),
        63,
        "circuit-level 12 neighbors"
      ) // 0.05ns
    )
    for ((config, vertexIndex, name) <- configurations) {
      val timingReport = Vivado.reportTiming(VertexResponse(config, vertexIndex))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
