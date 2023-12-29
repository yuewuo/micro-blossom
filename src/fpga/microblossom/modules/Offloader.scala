package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import org.scalatest.funsuite.AnyFunSuite

case class Offloader(config: DualConfig, offloaderIndex: Int, injectRegisters: Seq[String] = List()) extends Component {
  val io = new Bundle {}

  val stages = Stages(
    offload = () => new Bundle {},
    offload2 = () => new Bundle {},
    offload3 = () => new Bundle {},
    offload4 = () => StageOffloadOffloader4(config.numOffloaderNeighborOf(offloaderIndex))
  )

  // inject registers
  for (stageName <- injectRegisters) {
    stages.injectRegisterAt(stageName)
  }
  stages.finish()

}

// sbt 'testOnly microblossom.modules.OffloaderTest'
class OffloaderTest extends AnyFunSuite {

  test("construct a Offloader") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Offloader(config, 0))
  }

}
