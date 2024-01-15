package microblossom

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
import spinal.lib.bus.wishbone._
import spinal.lib.bus.regif._
import spinal.lib.bus.misc._
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
// 1. 128KB control block at [0, 0x1000]
//    0: (RO) 64 bits timer counter
//    8: (RO) 32 bits version register
//    12: (RO) 32 bits context depth
//    16: (RO) 8 bits number of obstacle channels (we're not using 100+ obstacle channels...)
//    20: (RW) 32 bits instruction counter
//  - (64 bits only) the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (WO) (32 bits instruction, 16 bits context id)
//    0x1008: (WO) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
//  - (32 bits only) the following 64KB section is designed for 32 bit bus where context id is encoded in the address
//    0x10000: 32 bits instruction for context 0
//    0x10004: 32 bits instruction for context 1
//    0x1FFFC: ... repeat for 65536: in total 64KB space
// 2. 2MB context readouts at [0x20_0000, 0x40_0000), each context is 4KB space
//    0: (RO) 64 bits obstacle timestamp
//    8: (RW) 32 bits grown value (for primal offloading)
//
//    32: (RO) 128 bits obstacle value [0]
//    : (RO) 128 bits obstacle value [1]
//    48: (RO) 128 bits obstacle value [2]
//       ...
case class MicroBlossom[T <: IMasterSlave, F <: BusSlaveFactoryDelayed](
    config: DualConfig,
    baseAddress: BigInt = 0,
    interfaceBuilder: () => T,
    slaveFactory: (T) => F
) extends Component {
  val io = new Bundle {
    val s0 = slave(interfaceBuilder())
  }

  val rawFactory = slaveFactory(io.s0)
  val factory = rawFactory.withOffset(baseAddress)

  require(factory.busDataWidth == 64 || factory.busDataWidth == 32, "only 64 bits or 32 bits bus is supported")
  val is64bus = factory.busDataWidth == 64

  // 0: (RO) 64 bits timer counter
  val counter = new Area {
    val value = Reg(UInt(64 bits)) init 0
    value := value + 1
    factory.readMultiWord(value, 0, documentation = "64 bits timer")
  }

  // 8: (RO) 32 bits version register
  // 12: (RO) 32 bits context depth
  // 16: (RO) 8 bits number of obstacle channels (we're not using 100+ obstacle channels...)
  val hardwareInfo = new Area {
    factory.readMultiWord(
      U(config.contextDepth, 32 bits) ## U(DualConfig.version, 32 bits),
      address = 8,
      documentation = "micro-blossom version and context depth"
    )
    factory.readMultiWord(
      U(config.obstacleChannels, 8 bits),
      address = 16,
      documentation = "the number of obtacle channels"
    )
    val instructionCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 24,
        documentation = "instruction counter"
      ) init (0)
  }

  val instruction = new Area {
    if (is64bus) {
      factory.onWritePrimitive(
        SizeMapping(base = 4096, size = 4096),
        haltSensitive = false,
        documentation = "instruction array (64 bits)"
      ) {
        hardwareInfo.instructionCounter := hardwareInfo.instructionCounter + 1
      }
    } else {
      factory.onWritePrimitive(
        SizeMapping(base = 65536, size = 65536),
        haltSensitive = false,
        documentation = "instruction array (32 bits)"
      ) {
        hardwareInfo.instructionCounter := hardwareInfo.instructionCounter + 1
      }
    }
  }

  rawFactory.printDataModel()

}

object MicroBlossomAxi4 {
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      axi4Config: Axi4Config = VersalAxi4Config(addressWidth = log2Up(4 MiB))
  ) = {
    MicroBlossom(
      config,
      baseAddress,
      () => Axi4SpecRenamer(Axi4(axi4Config)),
      (x: Axi4) => Axi4SlaveFactory(x)
    )
  }
}

object MicroBlossomAxiLite4 {
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      axiLite4Config: AxiLite4Config = AxiLite4Config(addressWidth = log2Up(4 MiB), dataWidth = 64)
  ) = {
    MicroBlossom(
      config,
      baseAddress,
      () => AxiLite4SpecRenamer(AxiLite4(axiLite4Config)),
      (x: AxiLite4) => AxiLite4SlaveFactory(x)
    )
  }
}

object MicroBlossomAxiLite4Bus32 {
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      axiLite4Config: AxiLite4Config = AxiLite4Config(addressWidth = log2Up(4 MiB), dataWidth = 32)
  ) = {
    MicroBlossom(
      config,
      baseAddress,
      () => AxiLite4SpecRenamer(AxiLite4(axiLite4Config)),
      (x: AxiLite4) => AxiLite4SlaveFactory(x)
    )
  }
}

// efabless uses 32 bits Wishbone interface, which is a lot simpler than AXI4
// https://caravel-harness.readthedocs.io/en/latest/
// https://caravel-mgmt-soc-litex.readthedocs.io/en/latest/
object MicroBlossomWishboneBus32 {
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      wishboneConfig: WishboneConfig = WishboneConfig(addressWidth = log2Up(4 MiB), dataWidth = 32)
  ) = {
    MicroBlossom(
      config,
      baseAddress,
      () => Wishbone(wishboneConfig),
      (x: Wishbone) => WishboneSlaveFactory(x)
    )
  }
}

// sbt 'testOnly *MicroBlossomTest'
class MicroBlossomTest extends AnyFunSuite {

  test("construct a MicroBlossom Module") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    config.sanityCheck()
    Config.spinal().generateVerilog(MicroBlossomAxi4(config))
  }

  test("logic validity") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")

    Config.sim
      .compile(MicroBlossomAxiLite4(config))
      .doSim("logic validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

      val driver = AxiLite4Driver(dut.io.s0, dut.clockDomain)
      driver.reset()

      val version = driver.read(8)
      printf("version: %x\n", version)
      }

  }

}

// sbt "runMain microblossom.MicroBlossomVerilog <config> <folder>"
// (e.g.) sbt "runMain microblossom.MicroBlossomVerilog ./resources/graphs/example_code_capacity_d3.json gen"
object MicroBlossomVerilog extends App {
  if (args.length != 2 && args.length != 3) {
    Console.err.println("usage: <config> <folder> [busType=Axi4|AxiLite4|..]")
    sys.exit(1)
  }
  val config = DualConfig(filename = args(0))
  var busType = "Axi4"
  if (args.length >= 3) {
    busType = args(2)
  }
  busType match {
    case "Axi4"          => Config.argFolderPath(args(1)).generateVerilog(MicroBlossomAxi4(config))
    case "AxiLite4"      => Config.argFolderPath(args(1)).generateVerilog(MicroBlossomAxiLite4(config))
    case "AxiLite4Bus32" => Config.argFolderPath(args(1)).generateVerilog(MicroBlossomAxiLite4Bus32(config))
    case "WishboneBus32" => Config.argFolderPath(args(1)).generateVerilog(MicroBlossomWishboneBus32(config))
    case _               => throw new Exception(s"bus type $busType is not recognized")
  }
}
