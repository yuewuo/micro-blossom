/*
 * # Micro Blossom Accelerator
 *
 * This module provides unified access to the Distributed Dual module with AXI4 interface.
 *
 *
 */

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.amba4.axilite.sim._
import spinal.lib.bus.regif._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._
import microblossom.modules._
import org.scalatest.funsuite.AnyFunSuite

case class MicroBlossom(config: DualConfig) extends Component {
  val io = new Bundle {
    val bus = slave(
      AxiLite4(
        AxiLite4Config(
          // max d=31 (31^3 < 32768), for 1% physical error rate we have 18 reported obstacles on average
          // 16 byte control (version 4, context depth 2, channels 1,), (instruction 4, context 2, length 2)
          // the rest are `config.obstacleChannels` separate memory spaces obstacles * 16 bytes each
          addressWidth = log2Up(448),
          dataWidth = 64 // for less transaction
        )
      )
    )
  }

  val busif = AxiLite4BusInterface(io.bus, (0x000, 16 Byte))

  // control fields
  val control = busif.newRegAt(address = 0, doc = "control fields")
  // note: although SpinalHDL recommend the use of `ROV`, it doesn't generate the name properly
  // thus I still use the old `RO` method
  control.field(32 bits, AccessType.RO, DualConfig.version, doc = "micro-blossom version")(
    SymbolName("version")
  ) := DualConfig.version
  control.field(16 bits, AccessType.RO, config.contextDepth, doc = "context depth")(
    SymbolName("context_depth")
  ) := config.contextDepth
  control.field(8 bits, AccessType.RO, config.obstacleChannels, doc = "the number of obtacle channels")(
    SymbolName("obstacle_channels")
  ) := config.obstacleChannels

  for (channelId <- 0 until config.obstacleChannels) {}

  def genDocs() = {
    busif.accept(CHeaderGenerator("MicroBlossom", "MicroBlossom"))
    busif.accept(HtmlGenerator("MicroBlossom", "Interupt Example"))
    busif.accept(JsonGenerator("MicroBlossom"))
    busif.accept(RalfGenerator("MicroBlossom"))
    busif.accept(SystemRdlGenerator("MicroBlossom", "MicroBlossom"))
  }

  this.genDocs()
}

// sbt 'testOnly *MicroBlossomTest'
class MicroBlossomTest extends AnyFunSuite {

  test("construct a MicroBlossom Module") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    config.sanityCheck()
    Config.spinal().generateVerilog(MicroBlossom(config))
  }

  test("logic validity") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")

    Config.sim
      .compile(MicroBlossom(config))
      .doSim("logic validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

        val driver = AxiLite4Driver(dut.io.bus, dut.clockDomain)
        driver.reset()

        val version = driver.read(0)
        printf("version: %x\n", version)
      }

  }

}
