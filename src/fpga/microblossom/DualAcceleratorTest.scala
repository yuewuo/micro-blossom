package microblossom

import spinal.core._
import org.scalatest.funsuite.AnyFunSuite

// sbt 'testOnly *DualAcceleratorTest'
class DualAcceleratorTest extends AnyFunSuite {

  test("construct accelerator from file") {
    val config = new DualConfig(filename = "./resources/graphs/example_repetition_code.json")
    // SpinalVerilog(new DualAccelerator(config))
    Config.spinal.generateVerilog(new DualAccelerator(config))
  }

}
