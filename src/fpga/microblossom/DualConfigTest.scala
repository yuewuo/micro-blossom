package microblossom

import spinal.core._
import org.scalatest.funsuite.AnyFunSuite

// sbt 'testOnly *DualConfigTest'
class DualConfigTest extends AnyFunSuite {

  test("construct config manually") {
    val config = DualConfig(vertexBits = 4, weightBits = 3)
    config.sanityCheck()
  }

  test("construct config incorrectly") {
    assertThrows[AssertionError] {
      // if the weight consists of too many bits to fit into a single message
      val config = new DualConfig(vertexBits = 4, weightBits = 10)
      config.sanityCheck()
    }
  }

  test("construct config from file") {
    val config = new DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    config.sanityCheck()
    assert(config.localIndexOfEdge(vertexIndex = 0, edgeIndex = 0) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 1, edgeIndex = 0) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 1, edgeIndex = 1) == 1)
    assert(config.localIndexOfEdge(vertexIndex = 2, edgeIndex = 1) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 3, edgeIndex = 2) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 0, edgeIndex = 2) == 1)
    assertThrows[Exception] { // exception when the edge is not incident to the vertex
      assert(config.localIndexOfEdge(vertexIndex = 0, edgeIndex = 1) == 0)
    }
  }

}
