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
// since there is no need to save memory space, we just allocate whatever convenient; for now we assume 8MB
// 1. 128KB control block at [0, 0x1000]
//    0: (RO) 64 bits timer counter
//    8: (RO) 32 bits version register
//    12: (RO) 32 bits context depth
//    16: (RO) 8 bits number of obstacle channels (we're not using 100+ obstacle channels...)
//    24: (RW) 32 bits instruction counter
//    32: (RW) 32 bits readout counter
//  - (64 bits only) the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (WO) (32 bits instruction, 16 bits context id)
//    0x1008: (WO) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
//  - (32 bits only) the following 64KB section is designed for 32 bit bus where context id is encoded in the address
//    0x10000: 32 bits instruction for context 0
//    0x10004: 32 bits instruction for context 1
//    0x1FFFC: ... repeat for 65536: in total 64KB space
// 2. 4MB context readouts at [0x40_0000, 0x80_0000), each context takes 1KB space, assuming no more than 4K contexts
//    [context 0]
//      0: (RO) 16 bits growable value (writing to this position has no effect)
//      2: (RW) 16 bits accumulated grown value (for primal offloading)
//      4: (RW) 16 bits maximum growth (offloaded primal), when 0, disable offloaded primal,
//                  write to this field will automatically clear accumulated grown value
//      (at most 62 concurrent obstacle report, already large enough)
//      32: (RO) 128 bits obstacle value [0] (96 bits obstacle value, 8 bits is_valid)
//      48: (RO) 128 bits obstacle value [1]
//      64: (RO) 128 bits obstacle value [2]
//         ...
//    [context 1]
//      1024: (RO) 32 bits growable value, when 0, the obstacle values are valid
//         ...
//
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
    val readoutCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 32,
        documentation = "readout counter"
      ) init (0)
  }

  // instantiate distributed dual
  val ioConfig = DualConfig()
  ioConfig.contextDepth = config.contextDepth
  ioConfig.weightBits = 16
  val dual = DistributedDual(config, ioConfig)
  dual.io.message.valid := False
  dual.io.message.assignDontCareToUnasigned()

  // keep track of some history to avoid data races
  val readoutLatency = config.readLatency + 1 // add 1 clock latency from the readout memory
  val initHistoryEntry = HistoryEntry(config)
  initHistoryEntry.valid := False
  initHistoryEntry.assignDontCareToUnasigned()
  require(readoutLatency >= 2)
  val historyEntries = Vec.fill(readoutLatency)(Reg(HistoryEntry(config)) init initHistoryEntry)
  // shift register
  for (i <- 0 until readoutLatency - 1) {
    historyEntries(i + 1) := historyEntries(i)
  }
  val nextHistoryEntry = HistoryEntry(config)
  nextHistoryEntry.valid := False
  nextHistoryEntry.assignDontCareToUnasigned()

  val instruction = new Area {
    val writeInstruction = Bits(32 bits)
    val writeContextId = UInt(16 bits)
    def onAskWrite() = {
      // block writing to avoid data race if there exists any writes within config.executeLatency
      val blockers = Vec.fill(config.executeLatency)(Bool)
      for (i <- 0 until config.executeLatency) {
        if (config.contextBits > 0) {
          blockers(i) := historyEntries(i).valid &&
            historyEntries(i).contextId === writeContextId.resize(config.contextBits)
        } else {
          blockers(i) := historyEntries(i).valid
        }
      }
      val isBlocked = Bool
      if (config.executeLatency > 0) {
        isBlocked := blockers.reduceBalancedTree(_ | _)
      } else {
        isBlocked := False
      }
      when(isBlocked) {
        factory.writeHalt()
      }
    }
    def onDoWrite() = {
      hardwareInfo.instructionCounter := hardwareInfo.instructionCounter + 1
      // execute the instruction
      nextHistoryEntry.valid := True
      if (config.contextBits > 0) {
        nextHistoryEntry.contextId := writeContextId.resize(config.contextBits)
      }
      // report(L"doing Write instruction = $writeInstruction, contextId = $writeContextId")
      dual.io.message.valid := True
      dual.io.message.instruction := writeInstruction
      if (config.contextBits > 0) {
        dual.io.message.contextId := writeContextId.resize(config.contextBits)
      }
    }
    if (is64bus) {
      val mapping = SizeMapping(base = 4 KiB, size = 4 KiB)
      val documentation = "instruction array (64 bits)"
      factory.nonStopWrite(writeInstruction, bitOffset = 0)
      factory.nonStopWrite(writeContextId, bitOffset = 32)
      factory.onWritePrimitive(mapping, haltSensitive = false, documentation)(onAskWrite)
      factory.onWritePrimitive(mapping, haltSensitive = true, documentation)(onDoWrite)
    } else {
      val mapping = SizeMapping(base = 64 KiB, size = 64 KiB)
      val documentation = "instruction array (32 bits)"
      factory.nonStopWrite(writeInstruction, bitOffset = 0)
      writeContextId := factory.writeAddress().resize(log2Up(64 KiB))
      factory.onWritePrimitive(mapping, haltSensitive = false, documentation)(onAskWrite)
      factory.onWritePrimitive(mapping, haltSensitive = true, documentation)(onDoWrite)
    }
  }

  historyEntries(0) := nextHistoryEntry

  // managing the context data from
  val context = new Area {
    val growable = Mem(UInt(16 bits), config.contextDepth)
    val maximumGrowth = Mem(UInt(16 bits), config.contextDepth)
    val accumulatedGrowth = Mem(UInt(16 bits), config.contextDepth)
    val conflicts = List.tabulate(config.obstacleChannels)(_ => {
      Mem(ConvergecastConflict(config.vertexBits), config.contextDepth)
    })

    val currentEntry = HistoryEntry(config)
    currentEntry := historyEntries(config.readLatency - 1)
    val currentId = if (config.contextBits > 0) {
      currentEntry.contextId
    } else { UInt(0 bits) }
    when(currentEntry.valid) {
      growable.write(currentId, dual.io.maxGrowable.length)
    }
  }

  val readouts = new Area {
    // each entry is 1KB
    val contextAddress: UInt = factory.readAddress().resize(10)
    val readContextId: UInt = (factory.readAddress() >> 10).resize(config.contextBits)
    // readout values
    val readValue = if (is64bus) { Bits(64 bits).assignDontCare() }
    else { Bits(32 bits).assignDontCare() }
    // report(L"readContextId = $readContextId")
    val contextGrowable = UInt(16 bits).assignDontCare()
    def onAskRead() = {
      val blockers = Vec.fill(readoutLatency)(Bool)
      for (i <- 0 until readoutLatency) {
        if (config.contextBits > 0) {
          blockers(i) := historyEntries(i).valid &&
            historyEntries(i).contextId === readContextId
        } else {
          blockers(i) := historyEntries(i).valid
        }
      }
      val isBlocked = Bool
      if (readoutLatency > 0) {
        isBlocked := blockers.reduceBalancedTree(_ | _)
      } else {
        isBlocked := False
      }
      // regardless of whether it's blocked, put the address in ram first so that it's ready the next cycle
      contextGrowable := context.growable.readSync(readContextId)
      when(isBlocked) {
        factory.readHalt()
      }
    }
    def onDoRead() = {
      hardwareInfo.readoutCounter := hardwareInfo.readoutCounter + 1
      if (is64bus) {
        when(contextAddress === 0) {
          readValue := U(0, 48 bits) ## contextGrowable.resize(16 bits)
        }
      } else {
        when(contextAddress === 0) {
          readValue := U(0, 16 bits) ## contextGrowable.resize(16 bits)
        }
      }

    }
    val mapping = SizeMapping(base = 4 MiB, size = 4 MiB)
    val documentation = "readout array (1 KB each, 4K of them at most)"
    factory.readPrimitive(readValue, mapping, 0, "readouts")
    factory.onReadPrimitive(mapping, haltSensitive = false, documentation)(onAskRead)
    factory.onReadPrimitive(mapping, haltSensitive = true, documentation)(onDoRead)
  }

  rawFactory.printDataModel()

}

case class HistoryEntry(config: DualConfig) extends Bundle {
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

object MicroBlossomAxi4 {
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      axi4Config: Axi4Config = VersalAxi4Config(addressWidth = log2Up(8 MiB))
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
  def renamedAxiLite4(config: AxiLite4Config) = {
    val axiLite4 = AxiLite4(config)
    AxiLite4SpecRenamer(axiLite4)
    axiLite4
  }
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      axiLite4Config: AxiLite4Config = AxiLite4Config(addressWidth = log2Up(8 MiB), dataWidth = 64)
  ) = {
    MicroBlossom[AxiLite4, AxiLite4SlaveFactory](
      config,
      baseAddress,
      () => renamedAxiLite4(axiLite4Config),
      (x: AxiLite4) => AxiLite4SlaveFactory(x)
    )
  }
}

object MicroBlossomAxiLite4Bus32 {
  def apply(
      config: DualConfig,
      baseAddress: BigInt = 0,
      axiLite4Config: AxiLite4Config = AxiLite4Config(addressWidth = log2Up(8 MiB), dataWidth = 32)
  ) = {
    MicroBlossom[AxiLite4, AxiLite4SlaveFactory](
      config,
      baseAddress,
      () => MicroBlossomAxiLite4.renamedAxiLite4(axiLite4Config),
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
      wishboneConfig: WishboneConfig = WishboneConfig(addressWidth = log2Up(8 MiB), dataWidth = 32)
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
      // .compile(MicroBlossomAxiLite4Bus32(config))
      .doSim("logic validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

        val driver = AxiLite4TypedDriver(dut.io.s0, dut.clockDomain)

        val version = driver.read_32(8)
        printf("version: %x\n", version)
        assert(version == DualConfig.version)
        val contextDepth = driver.read_32(12)
        assert(contextDepth == config.contextDepth)
        val obstacleChannels = driver.read_8(16)
        assert(obstacleChannels == config.obstacleChannels)
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
