package microblossom

import spinal.core._
import org.scalatest.funsuite.AnyFunSuite

// sbt 'testOnly *VertexTest'
class VertexTest extends AnyFunSuite {

  test("construct a Vertex") {
    val config = new DualConfig(filename = "./resources/graphs/example_repetition_code.json")
    config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.sanityCheck()
    assert(config.contextBits == 10); // 10 bit address
    Config.spinal.generateVerilog(Vertex(config, 0))
  }

}
