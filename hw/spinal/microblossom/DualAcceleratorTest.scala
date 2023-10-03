package microblossom

import spinal.core._
import org.scalatest.funsuite.AnyFunSuite

class DualConfigTest extends AnyFunSuite {

  test("construct config manually") {
    val config = DualConfig(VertexBits = 4, WeightBits = 3)
    config.sanity_check()
  }

  test("construct config incorrectly") {
    assertThrows[AssertionError] {
      // if the weight consists of too many bits to fit into a single message
      val config = DualConfig(VertexBits = 4, WeightBits = 10)
      config.sanity_check()
    }
  }

  test("construct config from file") {}

}
