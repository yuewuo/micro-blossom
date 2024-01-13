/*
 * # Micro Blossom Accelerator
 *
 * This module provides unified access to the Distributed Dual module with AXI4 interface.
 *
 *
 */

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axi._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.amba4.axilite.sim._
import spinal.lib.bus.regif._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._
import microblossom.modules._
import org.scalatest.funsuite.AnyFunSuite
import scala.collection.mutable.ArrayBuffer

/*
 * A single MicroBlossom module can be virtualized as multiple modules,
 * each of them has a unique base address that can access the full functions as if it occupies the hardware.
 *
 * One can simply understand MicroBlossom as in-memory computation.
 * Each physical memory represents a single context; Virtualization allows one to allocate memory resources
 * and share the logic resources via time sharing.
 *
 * A hypervisor should be introduced if clients are not trusted, however that seems to be overkill for now.
 */
case class VirtualMicroBlossom() extends Bundle {
  val instruction = Bits(32 bits)
  val obstacle = Bits(96 bits)
  val grown = Bits(16 bits)
  val status = Bits(16 bits)
}

case class MicroBlossom(config: DualConfig) extends Component {
  val io = new Bundle {
    val bus = slave(
      Axi4(
        Axi4Config(
          // max d=31 (31^3 < 32768), for 1% physical error rate we have 18 reported obstacles on average
          // 16 byte control
          //   (version 4, context depth 2, #channels 1,)
          //   64-bit timer
          // 16 byte message
          //   (instruction 4, context_id 2)
          //   (status 1, grown length: 2, context_id 2)
          //
          // following the vector values for each context entry
          //   4 control (status 1, )
          //   16 byte: 12 byte obstacle + 2 byte grown
          // the rest are `config.obstacleChannels` separate memory spaces obstacles * 16 bytes each
          addressWidth = log2Up(32 + 32 * config.obstacleChannels),
          dataWidth = 64, // for less transaction
          useId = false, // no need for now
          useQos = false
        )
      )
    )
  }

  val factory = Axi4SlaveFactory(io.bus)

  // val busif = AxiLite4BusInterface(io.bus, (0x000, 32 Byte))
  // // control register
  // val control = busif.newRegAt(address = 0, doc = "control registers")
  // // note: although SpinalHDL recommend the use of `ROV`, it doesn't generate the name properly
  // // thus I still use the old `RO` method
  // control.field(32 bits, AccessType.RO, DualConfig.version, doc = "micro-blossom version")(
  //   SymbolName("version")
  // ) := DualConfig.version
  // control.field(16 bits, AccessType.RO, config.contextDepth, doc = "context depth")(
  //   SymbolName("context_depth")
  // ) := config.contextDepth
  // control.field(8 bits, AccessType.RO, config.obstacleChannels, doc = "the number of obtacle channels")(
  //   SymbolName("obstacle_channels")
  // ) := config.obstacleChannels

  // // timer register
  // val timerReg = busif.newRegAt(address = 8, doc = "64-bit timer")
  // val
  // timerReg.field(32 bits, AccessType.RO, DualConfig.version, doc = "micro-blossom version")(
  //   SymbolName("version")
  // ) := DualConfig.version

  // val obstacles = ArrayBuffer[Bits]()
  // for (channelId <- 0 until config.obstacleChannels) {
  //   val obstacle_upper =
  //     busif.newRegAt(address = 32 + channelId * 16, doc = s"obstacle $channelId upper half")(
  //       SymbolName(s"obstacle_${channelId}_upper")
  //     )
  //   val obstacle_lower =
  //     busif.newRegAt(address = 32 + channelId * 16 + 8, doc = s"obstacle $channelId lower half")(
  //       SymbolName(s"obstacle_${channelId}_lower")
  //     )
  //   val obstacle = Reg(Bits(128 bit)) init 0
  //   obstacle := obstacle | busif.writeData.resized
  //   obstacle_upper.field(64 bits, AccessType.RO)(SymbolName("value")) := obstacle(127 downto 64)
  //   obstacle_lower.field(64 bits, AccessType.RO)(SymbolName("value")) := obstacle(63 downto 0)
  //   obstacles.append(obstacle)
  // }

  // def genDocs() = {
  //   busif.accept(CHeaderGenerator("MicroBlossom", "MicroBlossom"))
  //   busif.accept(HtmlGenerator("MicroBlossom", "MicroBlossom"))
  //   busif.accept(JsonGenerator("MicroBlossom"))
  //   // busif.accept(RalfGenerator("MicroBlossom"))
  //   // busif.accept(SystemRdlGenerator("MicroBlossom", "MicroBlossom"))
  // }

  // this.genDocs()

  Axi4SpecRenamer(io.bus)
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

      // val driver = AxiLite4Driver(dut.io.bus, dut.clockDomain)
      // driver.reset()

      // val version = driver.read(0)
      // printf("version: %x\n", version)
      }

  }

}
