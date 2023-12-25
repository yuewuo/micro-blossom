package microblossom.vertex

import spinal.core._
import spinal.lib._
import microblossom._
import org.scalatest.funsuite.AnyFunSuite

case class VertexPersistent(config: DualConfig, vertexIndex: Int) extends Bundle {
  val speed = Speed()
  val node = Bits(config.vertexBits bits)
  val root = Bits(config.vertexBits bits)
  val isVirtual = Bool
  val isDefect = Bool
  val grown = UInt(config.grownBitsOf(vertexIndex) bits)
}

case class Vertex(config: DualConfig, vertexIndex: Int) extends Component {
  val io = new Bundle {
    val input = in(BroadcastMessage(config))
  }

  // val fetch = new Area {
  //   val
  // }

}

// sbt 'testOnly microblossom.vertex.VertexTest'
class VertexTest extends AnyFunSuite {

  test("construct a Vertex") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Vertex(config, 0))
  }

}
