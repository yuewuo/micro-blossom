package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

case class VertexShadowResult(vertexBits: Int) extends Bundle {
  val speed = Speed()
  val node = Bits(vertexBits bits)
  val root = Bits(vertexBits bits)
}

object VertexShadow {
  def build(
      shadow: VertexShadowResult, // output
      node: Bits,
      root: Bits,
      speed: Speed,
      grown: UInt,
      isStalled: Bool,
      propagator: VertexPropagatingPeerResult
  ) = {

    // default to the original state
    shadow.speed := speed
    shadow.node := node
    shadow.root := root

    // consider the propagator
    when(speed === Speed.Shrink && grown === 0) {
      when(propagator.valid) {
        shadow.node := propagator.node
        shadow.root := propagator.root
        shadow.speed := Speed.Grow
      }
    }
    when(isStalled) {
      shadow.speed := Speed.Stay
    }
  }
}

case class VertexShadow(config: DualConfig, vertexIndex: Int) extends Component {
  val grownBits = config.grownBitsOf(vertexIndex)

  val io = new Bundle {
    val node = in(Bits(config.vertexBits bits))
    val root = in(Bits(config.vertexBits bits))
    val speed = in(Speed())
    val grown = in(UInt(grownBits bits))
    val isStalled = in(Bool)

    val propagator = in(VertexPropagatingPeerResult(config.vertexBits))

    val shadow = out(VertexShadowResult(config.vertexBits))
  }

  VertexShadow.build(
    io.shadow,
    io.node,
    io.root,
    io.speed,
    io.grown,
    io.isStalled,
    io.propagator
  )
}

// sbt 'testOnly microblossom.combinatorial.VertexShadowTest'
class VertexShadowTest extends AnyFunSuite {

  test("example") {
    val config = DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json")
    val vertexIndex = 3
    Config.spinal().generateVerilog(VertexShadow(config, vertexIndex))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexShadowEstimation'
class VertexShadowEstimation extends AnyFunSuite {

  test("logic delay") {
    def dualConfig(name: String): DualConfig = {
      DualConfig(filename = s"./resources/graphs/example_$name.json"),
    }
    val configurations = List(
      // delay: 0.41ns
      // resource: 1xLUT5, 12xLUT4 -> 13
      (dualConfig("code_capacity_d5"), 1, "code capacity 2 neighbors"),
      // delay: 0.41ns
      // resource: 1xLUT5, 14xLUT4 -> 15
      (dualConfig("code_capacity_rotated_d5"), 10, "code capacity 4 neighbors"),
      // delay: 0.42ns
      // resource: 1xLUT5, 18xLUT4 -> 19
      (dualConfig("phenomenological_rotated_d5"), 64, "phenomenological 6 neighbors"),
      // delay: 0.42ns
      // resource: 18xLUT6, 3xLUT5 -> 21
      (dualConfig("circuit_level_d5"), 63, "circuit-level 12 neighbors")
    )
    for ((config, vertexIndex, name) <- configurations) {
      val reports = Vivado.report(VertexShadow(config, vertexIndex))
      println(s"$name: ${reports.timing.getPathDelaysExcludingIOWorst}ns")
      reports.resource.primitivesTable.print()
    }
  }

}
