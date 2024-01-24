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
import microblossom.driver._
import microblossom.types._
import microblossom.modules._
import microblossom.stage._
import org.scalatest.funsuite.AnyFunSuite
import scala.collection.mutable.ArrayBuffer
import org.rogach.scallop._

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
//    16: (RO) 8 bits number of conflict channels (we're not using 100+ conflict channels...)
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
//      0: (RW) 16 bits maximum growth (offloaded primal), when 0, disable offloaded primal,
//                  write to this field will automatically clear accumulated grown value
//      2: (RW) 16 bits accumulated grown value (for primal offloading)
//      4: (RO) 16 bits growable value (writing to this position has no effect)
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
    dualConfig: DualConfig,
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
  // 16: (RO) 8 bits number of conflict channels (we're not using 100+ conflict channels...)
  val hardwareInfo = new Area {
    factory.readMultiWord(
      U(dualConfig.contextDepth, 32 bits) ## U(DualConfig.version, 32 bits),
      address = 8,
      documentation = "micro-blossom version and context depth"
    )
    factory.readMultiWord(
      U(dualConfig.conflictChannels, 8 bits),
      address = 16,
      documentation = "the number of conflict channels"
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
  ioConfig.contextDepth = dualConfig.contextDepth
  ioConfig.weightBits = dualConfig.weightBits
  ioConfig.vertexBits = dualConfig.vertexBits
  val dual = DistributedDual(dualConfig, ioConfig)
  dual.io.message.valid := False
  dual.io.message.assignDontCareToUnasigned()

  // keep track of some history to avoid data races
  require(dualConfig.readLatency >= 2)
  val readoutLatency = dualConfig.readLatency + 1 // add 1 clock latency from the readout memory
  val initHistoryEntry = HistoryEntry(dualConfig)
  initHistoryEntry.valid := False
  initHistoryEntry.assignDontCareToUnasigned()
  val historyEntries = Vec.fill(readoutLatency)(Reg(HistoryEntry(dualConfig)) init initHistoryEntry)
  // shift register
  for (i <- 0 until readoutLatency - 1) {
    historyEntries(i + 1) := historyEntries(i)
  }
  val nextHistoryEntry = HistoryEntry(dualConfig)
  nextHistoryEntry.valid := False
  nextHistoryEntry.assignDontCareToUnasigned()

  val primalOffloadIssuing = Bool
  primalOffloadIssuing := False
  val instruction = new Area {
    val writeInstruction = Instruction(DualConfig())
    require(writeInstruction.getBitsWidth == 32)
    val writeContextId = UInt(16 bits)
    val isAskingWrite = Bool
    isAskingWrite := False
    def onAskWrite() = {
      isAskingWrite := True
      // block writing to avoid data race if there exists any writes within config.executeLatency
      val blockers = Vec.fill(dualConfig.executeLatency)(Bool)
      for (i <- 0 until dualConfig.executeLatency) {
        if (dualConfig.contextBits > 0) {
          blockers(i) := historyEntries(i).valid &&
            historyEntries(i).contextId === writeContextId.resize(dualConfig.contextBits)
        } else {
          blockers(i) := historyEntries(i).valid
        }
      }
      val isBlocked = Bool
      if (dualConfig.executeLatency > 0) {
        isBlocked := blockers.reduceBalancedTree(_ | _)
      } else {
        isBlocked := False
      }
      when(isBlocked || primalOffloadIssuing) {
        factory.writeHalt()
      }
    }
    val isDoingWrite = Bool
    isDoingWrite := False
    def onDoWrite() = {
      isDoingWrite := True
      hardwareInfo.instructionCounter := hardwareInfo.instructionCounter + 1
      // execute the instruction
      nextHistoryEntry.valid := True
      if (dualConfig.contextBits > 0) {
        nextHistoryEntry.contextId := writeContextId.resize(dualConfig.contextBits)
      }
      // report(L"doing Write instruction = $writeInstruction, contextId = $writeContextId")

      dual.io.message.valid := True
      dual.io.message.instruction.resizedFrom(writeInstruction)
      if (dualConfig.contextBits > 0) {
        dual.io.message.contextId := writeContextId.resize(dualConfig.contextBits)
      }
    }
    val (mapping, documentation) = if (is64bus) {
      factory.nonStopWrite(writeInstruction, bitOffset = 0)
      factory.nonStopWrite(writeContextId, bitOffset = 32)
      (SizeMapping(base = 4 KiB, size = 4 KiB), "instruction array (64 bits)")
    } else {
      factory.nonStopWrite(writeInstruction, bitOffset = 0)
      writeContextId := factory.writeAddress().resize(log2Up(64 KiB))
      (SizeMapping(base = 64 KiB, size = 64 KiB), "instruction array (32 bits)")
    }
    factory.onWritePrimitive(mapping, haltSensitive = false, documentation)(onAskWrite)
    factory.onWritePrimitive(mapping, haltSensitive = true, documentation)(onDoWrite)
  }

  historyEntries(0) := nextHistoryEntry

  // managing the context data from
  val context = new Area {
    val growable = OneMem(ConvergecastMaxGrowable(dualConfig.weightBits), dualConfig.contextDepth)
    val maximumGrowth =
      OneMem(UInt(16 bits), dualConfig.contextDepth) init List.fill(dualConfig.contextDepth)(U(0, 16 bits))
    val accumulatedGrown =
      OneMem(UInt(16 bits), dualConfig.contextDepth) init List.fill(dualConfig.contextDepth)(U(0, 16 bits))
    val conflicts = List.tabulate(dualConfig.conflictChannels)(_ => {
      OneMem(ConvergecastConflict(dualConfig.vertexBits), dualConfig.contextDepth)
    })

    val currentEntry = HistoryEntry(dualConfig)
    currentEntry := historyEntries(dualConfig.readLatency - 1)
    val currentMaximumGrowth = UInt(16 bits)
    val currentAccumulatedGrown = UInt(16 bits)
    val nextEntry = HistoryEntry(dualConfig)
    if (dualConfig.readLatency >= 2) {
      nextEntry := historyEntries(dualConfig.readLatency - 2)
    } else {
      assert(dualConfig.readLatency == 2)
      nextEntry := nextHistoryEntry
    }
    val nextId = if (dualConfig.contextBits > 0) {
      nextEntry.contextId
    } else { UInt(0 bits) }
    currentMaximumGrowth := maximumGrowth.readSync(nextId)
    currentAccumulatedGrown := accumulatedGrown.readSync(nextId)
    val currentId = if (dualConfig.contextBits > 0) {
      currentEntry.contextId
    } else { U(0, 0 bits) }
    when(currentEntry.valid) {
      growable.write(currentId, dual.io.maxGrowable)
      for (i <- 0 until dualConfig.conflictChannels) {
        conflicts(i).write(currentId, dual.io.conflict) // TODO: implement real multi-channel conflict reporting
      }
      val maxGrowable = dual.io.maxGrowable.length
      when(maxGrowable =/= 0 && maxGrowable =/= maxGrowable.maxValue) {
        when(currentMaximumGrowth > currentAccumulatedGrown) {
          primalOffloadIssuing := True // half the bus if writing instruction
          // write a Grow instruction
          nextHistoryEntry.valid := True
          if (dualConfig.contextBits > 0) {
            nextHistoryEntry.contextId := currentId
          }
          val instruction = InstructionSpec
          val maxLength = UInt(16 bits)
          maxLength := currentMaximumGrowth - currentAccumulatedGrown
          val maxLengthTruncate = UInt(dualConfig.weightBits bits)
          when(maxLength > maxLengthTruncate.maxValue) {
            maxLengthTruncate := maxLengthTruncate.maxValue
          } otherwise {
            maxLengthTruncate := maxLength.resized
          }
          val length = UInt(dualConfig.weightBits bits)
          when(maxLengthTruncate > maxGrowable) {
            length := maxGrowable
          } otherwise {
            length := maxLengthTruncate
          }
          accumulatedGrown.write(currentId, currentAccumulatedGrown + length)
          val spec = InstructionSpec(ioConfig)
          dual.io.message.valid := True
          dual.io.message.instruction := (spec.generateExtendedSuffix(ExtendedOpCode.Grow) |
            spec.lengthRange.dynMasked(length.asBits)).resized
          if (dualConfig.contextBits > 0) {
            dual.io.message.contextId := currentId
          }
        }
      }
    }
  }

  val readouts = new Area {
    // each entry is 1KB
    val contextAddress: UInt = factory.readAddress().resize(10)
    val readContextId: UInt = (factory.readAddress() >> 10).resize(dualConfig.contextBits)
    val previousAskRead = Reg(Bool) init False
    previousAskRead := False
    // readout values
    val readValue = if (is64bus) { Bits(64 bits).assignDontCare() }
    else { Bits(32 bits).assignDontCare() }
    // report(L"readContextId = $readContextId")
    val contextGrowable = ConvergecastMaxGrowable(dualConfig.weightBits)
    val contextMaximumGrowth = UInt(16 bits)
    val contextAccumulatedGrown = UInt(16 bits)
    val contextConflicts = List.tabulate(dualConfig.conflictChannels)(_ => {
      ConvergecastConflict(dualConfig.vertexBits)
    })
    val writeMaximumGrowth = UInt(16 bits)
    val isWritingMaximumGrowth = Bool
    isWritingMaximumGrowth := False
    // use readFirst policy to avoid Vivado warnings about data corruption
    // regardless of whether it's blocked, put the address in ram first so that it's ready the next cycle
    contextGrowable := context.growable.readSync(readContextId)
    contextAccumulatedGrown := context.accumulatedGrown.readWriteSync(
      readContextId,
      U(0, 16 bits),
      enable = True,
      write = isWritingMaximumGrowth
    )
    contextMaximumGrowth := context.maximumGrowth.readWriteSync(
      readContextId,
      writeMaximumGrowth,
      enable = True,
      write = isWritingMaximumGrowth
    )
    for (index <- 0 until dualConfig.conflictChannels) {
      contextConflicts(index) := context.conflicts(index).readSync(readContextId)
    }
    // calculate whether it's blocked
    val blockers = Vec.fill(readoutLatency)(Bool)
    for (i <- 0 until readoutLatency) {
      if (dualConfig.contextBits > 0) {
        blockers(i) := historyEntries(i).valid &&
          historyEntries(i).contextId === readContextId
      } else {
        blockers(i) := historyEntries(i).valid
      }
    }
    val isBlocked = Bool
    isBlocked := blockers.reduceBalancedTree(_ | _)
    val isAskingRead = Bool
    isAskingRead := False
    def onAskRead() = {
      isAskingRead := True
      previousAskRead := True
      when(isBlocked || !previousAskRead) { // always halt for a clock cycle if previous cycle is not asking read
        factory.readHalt()
      }
    }
    val isDoingRead = Bool
    isDoingRead := False
    def onDoRead() = {
      isDoingRead := True
      hardwareInfo.readoutCounter := hardwareInfo.readoutCounter + 1
      // head
      val resizedContextGrowable = ConvergecastMaxGrowable(16)
      resizedContextGrowable.resizedFrom(contextGrowable)
      if (is64bus) {
        when(contextAddress === 0) {
          readValue := U(0, 16 bits) ## resizedContextGrowable.length ## contextAccumulatedGrown ##
            contextMaximumGrowth
        }
      } else {
        when(contextAddress === 0) {
          readValue := contextAccumulatedGrown ## contextMaximumGrowth
        }
        when(contextAddress === 4) {
          readValue := U(0, 16 bits) ## resizedContextGrowable.length
        }
      }
      // conflicts
      for (index <- 0 until dualConfig.conflictChannels) {
        val conflict = contextConflicts(index)
        val resizedConflict = ConvergecastConflict(16)
        resizedConflict.resizedFrom(conflict)
        val base = 32 + 16 * index
        if (is64bus) {
          when(contextAddress === base) {
            readValue := resizedConflict.touch2 ## resizedConflict.touch1 ##
              resizedConflict.node2 ## resizedConflict.node1
          }
          when(contextAddress === base + 8) {
            readValue := U(0, 24 bits) ## U(0, 7 bits) ## resizedConflict.valid ##
              resizedConflict.vertex2 ## resizedConflict.vertex1
          }
        } else {
          when(contextAddress === base) {
            readValue := resizedConflict.node2 ## resizedConflict.node1
          }
          when(contextAddress === base + 4) {
            readValue := resizedConflict.touch2 ## resizedConflict.touch1
          }
          when(contextAddress === base + 8) {
            readValue := resizedConflict.vertex2 ## resizedConflict.vertex1
          }
          when(contextAddress === base + 12) {
            readValue := U(0, 24 bits) ## U(0, 7 bits) ## resizedConflict.valid
          }
        }
      }
    }
    val isAskingWrite = Bool
    isAskingWrite := False
    def onAskWrite() = {
      isAskingWrite := True
      when(isBlocked) {
        factory.writeHalt()
      }
    }
    val isDoingWrite = Bool
    isDoingWrite := False
    def onDoWrite() = {
      isDoingWrite := True
      when(factory.writeAddress().resize(10) === 0) {
        isWritingMaximumGrowth := True
      }
    }
    val mapping = SizeMapping(base = 4 MiB, size = 4 MiB)
    val documentation = "readout array (1 KB each, 4K of them at most)"
    factory.readPrimitive(readValue, mapping, 0, "readouts")
    factory.onReadPrimitive(mapping, haltSensitive = false, documentation)(onAskRead)
    factory.onReadPrimitive(mapping, haltSensitive = true, documentation)(onDoRead)
    factory.nonStopWrite(writeMaximumGrowth, bitOffset = 0)
    factory.onWritePrimitive(mapping, haltSensitive = false, documentation)(onAskWrite)
    factory.onWritePrimitive(mapping, haltSensitive = true, documentation)(onDoWrite)
  }

  // rawFactory.printDataModel()

  def getSimDriver(): TypedDriver = {
    if (io.s0.isInstanceOf[AxiLite4]) {
      AxiLite4TypedDriver(io.s0.asInstanceOf[AxiLite4], clockDomain)
    } else if (io.s0.isInstanceOf[Axi4]) {
      Axi4TypedDriver(io.s0.asInstanceOf[Axi4], clockDomain)
    } else {
      throw new Exception("simulater driver not implemented")
    }
  }

}

case class HistoryEntry(config: DualConfig) extends Bundle {
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
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

        val driver = dut.getSimDriver()

        val version = driver.read_32(8)
        printf("version: %x\n", version)
        assert(version == DualConfig.version)
        val contextDepth = driver.read_32(12)
        assert(contextDepth == config.contextDepth)
        val conflictChannels = driver.read_8(16)
        assert(conflictChannels == config.conflictChannels)
      }

  }

}

class MicroBlossomGeneratorConf(arguments: Seq[String]) extends ScallopConf(arguments) {
  val graph = opt[String](required = true, descr = "see ./resources/graphs/README.md for more details")
  val outputDir = opt[String](default = Some("gen"), descr = "by default generate the output at ./gen")
  val busType = opt[String](default = Some("Axi4"), descr = s"options: ${MicroBlossomBusType.options.mkString(", ")}")
  val languageHdl = opt[String](default = Some("verilog"), descr = "options: Verilog, VHDL, SystemVerilog")
  val baseAddress = opt[BigInt](default = Some(0), descr = "base address of the memory-mapped module, default to 0")
  // DualConfig
  val broadcastDelay = opt[Int](default = Some(1))
  val convergecastDelay = opt[Int](default = Some(1))
  val contextDepth = opt[Int](default = Some(1), descr = "how many contexts supported")
  val conflictChannels = opt[Int](default = Some(1), descr = "how many conflicts are reported at once")
  val supportAddDefectVertex = opt[Boolean](descr = "support AddDefectVertex instruction")
  val injectRegisters =
    opt[List[String]](
      default = Some(List()),
      descr = s"insert register at select stages: ${Stages().stageNames.mkString(", ")}"
    )
  verify()
}

// sbt "runMain microblossom.MicroBlossomGenerator --help"
// (e.g.) sbt "runMain microblossom.MicroBlossomGenerator --graph ./resources/graphs/example_code_capacity_d3.json"
object MicroBlossomGenerator extends App {
  val conf = new MicroBlossomGeneratorConf(args)
  val config = DualConfig(
    filename = conf.graph(),
    broadcastDelay = conf.broadcastDelay(),
    convergecastDelay = conf.convergecastDelay(),
    contextDepth = conf.contextDepth(),
    conflictChannels = conf.conflictChannels(),
    supportAddDefectVertex = conf.supportAddDefectVertex(),
    injectRegisters = conf.injectRegisters()
  )
  val genConfig = Config.argFolderPath(conf.outputDir())
  // note: deliberately not creating `component` here, otherwise it encounters null pointer error of GlobalData.get()....
  conf.languageHdl() match {
    case "verilog" | "Verilog" =>
      genConfig.generateVerilog(MicroBlossomBusType.generateByName(conf.busType(), config, conf.baseAddress()))
    case "VHDL" | "vhdl" | "Vhdl" =>
      genConfig.generateVhdl(MicroBlossomBusType.generateByName(conf.busType(), config, conf.baseAddress()))
    case "SystemVerilog" | "systemverilog" =>
      genConfig.generateSystemVerilog(MicroBlossomBusType.generateByName(conf.busType(), config, conf.baseAddress()))
    case _ => throw new Exception(s"HDL language ${conf.languageHdl()} is not recognized")
  }
}
