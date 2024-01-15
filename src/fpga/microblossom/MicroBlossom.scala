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

// max d=31 (31^3 < 32768), for 0.1% physical error rate we have 18 reported obstacles on average
// since there is no need to save memory space (256MB), we just allocate whatever convenient
// 1. 8KB control block at [0, 0x1000]
//    0: (RO) 64 bits timer counter
//    8: (RO) 32 bits version register
//    12: (RO) 32 bits context depth
//    16: (RO) 8 bits number of obstacle channels (we're not using 100+ obstacle channels...)
//    24: (RW) (32 bits instruction, 16 bits context id)
//    the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (RW) (32 bits instruction, 16 bits context id)
//    0x1008: (RW) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
// 2. 2MB context readouts at [0x20_0000, 0x40_0000), each context is 4KB space
//    0: (RO) 64 bits obstacle timestamp
//    8: (RW) 32 bits grown value (for primal offloading)
//
//    32: (RO) 128 bits obstacle value [0]
//    : (RO) 128 bits obstacle value [1]
//    48: (RO) 128 bits obstacle value [2]
//       ...
case class MicroBlossom(
    config: DualConfig,
    baseAddress: BigInt = 0,
    axi4Config: Axi4Config = VersalAxi4Config(addressWidth = log2Up(4 MiB))
) extends Component {
  val io = new Bundle {
    val s0 = slave(Axi4(axi4Config))
  }

  val factory = Axi4SlaveFactory(io.s0)

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

  Axi4SpecRenamer(io.s0)
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

// sbt "runMain MicroBlossomVerilog <config> <folder>"
// e.g. sbt "runMain MicroBlossomVerilog ./resources/graphs/example_code_capacity_d3.json gen"
object MicroBlossomVerilog extends App {
  if (args.length != 2) {
    Console.err.println("usage: <config> <folder>")
    sys.exit(1)
  }
  val config = DualConfig(filename = args(0))
  Config.argFolderPath(args(1)).generateVerilog(MicroBlossom(config))
}
