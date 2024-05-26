package microblossom

/*
 * # Micro Blossom Accelerator
 *
 * This module provides unified access to the Distributed Dual module with AXI4 interface.
 *
 *
 * Note:
 *     1. When you issue instructions, by default we set maximumGrowth to 0 to avoid any spontaneous growth
 *        between the execution of your instructions. However, if you explicitly issue a `FindObstacle` instruction,
 *        we will use the `maximumGrowth` value you set last time. This is useful when you have context switching
 *        because you can issue `FindObstacle` and then the readout value will be cached so that it will be fast the
 *        the next time you actually read the obstacle.
 *
 */

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import spinal.core._
import spinal.lib._
import spinal.lib.fsm._
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
import org.rogach.scallop._

// max d=31 (31^3 < 32768), for 0.1% physical error rate we have 18 reported obstacles on average
// since there is no need to save memory space, we just allocate whatever convenient; for now we assume 8MB
// 1. 128KB control block at [0, 0x2_0000]
//    0: (RO) 64 bits timer counter
//    8: (RO) 32 bits version register
//    12: (RO) 32 bits context depth
//    16: (RO) 8 bits number of conflict channels (no more than 6 is supported)
//    17: (RO) 8 bits dualConfig.vertexBits
//    18: (RO) 8 bits dualConfig.weightBits
//    24: (RW) 32 bits instruction counter
//    32: (RW) 32 bits readout counter
//    40: (RW) 32 bits transaction counter
//    48: (RW) 32 bits error counter
//  - (64 bits only) the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (WO) (32 bits instruction, 16 bits context id)
//    0x1008: (WO) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
//    0x1FFC
//  - (32 bits only) the following 4KB section is designed for 32 bit bus where context id is encoded in the address
//    0x2000: 32 bits instruction for context 0
//    0x2004: 32 bits instruction for context 1
//    0x2008: ... repeat for 1024: in total 4KB space
//    0x2FFC
// 2. 128KB context readouts at [0x2_0000, 0x4_0000), each context takes 128 byte space, assuming no more than 1024 contexts
//    [context 0]
//      0: (RW) 64 bits timestamp of receiving the last ``load obstacles'' instruction
//      8: (RW) 64 bits timestamp of receiving the last ``growable = infinity'' response
//      16: (W) 16 bits maximum growth write
//      16: (R) head + conflict (max_growth: u16, accumulated: u16, growable: u16, conflict_valid: u8, conflict: 96 bits)
//            16 bits maximum growth (offloaded primal), when 0, disable offloaded primal,
//                  write to this field will automatically clear accumulated grown value
//            16 bits accumulated grown value (for primal offloading)
//            16 bits growable value (writing to this position has no effect)
//      (at most 15 concurrent conflict report, large enough)
//      32: next obstacle, the head remains the same
//        ...
//    [context 1]
//      128: ...
//

case class MicroBlossomBus[T <: IMasterSlave, F <: BusSlaveFactoryDelayed](
    config: DualConfig,
    clockDivideBy: Int = 2, // divided clock at io.dividedClock; note the clock must be synchronous and 0 phase aligned
    baseAddress: BigInt = 0,
    interfaceBuilder: () => T,
    slaveFactory: (T) => F
) extends Component {
  val io = new Bundle {
    val s0 = slave(interfaceBuilder())
    val slowClk = in Bool ()
  }

  val slowClk = io.slowClk
  slowClk.setName("slow_clk")

  val rawFactory = slaveFactory(io.s0)
  val factory = rawFactory.withOffset(baseAddress)

  require(clockDivideBy >= 2)
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
      U(config.contextDepth, 32 bits) ## U(DualConfig.version, 32 bits),
      address = 8,
      documentation = "micro-blossom version and context depth"
    )
    factory.readMultiWord(
      U(config.weightBits, 8 bits) ## U(config.vertexBits, 8 bits) ## U(config.conflictChannels, 8 bits),
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
    val transactionCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 40,
        documentation = "number of AXI4 transactions"
      ) init (0)
    val errorCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 48,
        documentation = "error counter"
      ) init (0)
  }
  val hasError = Bool
  hasError := False
  when(hasError) {
    hardwareInfo.errorCounter := hardwareInfo.errorCounter + 1
  }

  val slowClockDomain = ClockDomain(
    clock = slowClk,
    reset = ClockDomain.current.readResetWire,
    config = ClockDomainConfig(
      clockEdge = RISING,
      resetKind = SYNC,
      resetActiveLevel = HIGH
    )
  )

  val ccFifoPush = StreamFifoCC(
    dataType = LooperInput(config),
    depth = config.instructionBufferDepth,
    pushClock = clockDomain,
    popClock = slowClockDomain
  )
  val ccFifoPop = StreamFifoCC(
    dataType = LooperOutput(config),
    depth = config.instructionBufferDepth,
    pushClock = slowClockDomain,
    popClock = clockDomain
  )
  val slow = new ClockingArea(slowClockDomain) {
    val microBlossom = MicroBlossomLooper(config)
    microBlossom.io.push << ccFifoPush.io.pop
    microBlossom.io.pop >> ccFifoPop.io.push
  }
  def microBlossom = slow.microBlossom
  ccFifoPush.io.push.valid := False
  ccFifoPush.io.push.payload.assignDontCare()
  ccFifoPop.io.pop.ready := True

  // create the control registers
  val maximumGrowth = OneMem(UInt(16 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 16 bits))
  val accumulatedGrown = OneMem(UInt(16 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 16 bits))
  val maxGrowable = OneMem(ConvergecastMaxGrowable(config.weightBits), config.contextDepth)
  val conflicts = List.tabulate(config.conflictChannels)(_ => {
    OneMem(ConvergecastConflict(config.vertexBits), config.contextDepth)
  })
  val pushId = OneMem(UInt(config.instructionBufferBits bits), config.contextDepth) init
    List.fill(config.contextDepth)(U(0, config.instructionBufferBits bits))
  val popId = OneMem(UInt(config.instructionBufferBits bits), config.contextDepth) init
    List.fill(config.contextDepth)(U(0, config.instructionBufferBits bits))

  def getSimDriver(): TypedDriver = {
    if (io.s0.isInstanceOf[AxiLite4]) {
      AxiLite4TypedDriver(io.s0.asInstanceOf[AxiLite4], clockDomain)
    } else if (io.s0.isInstanceOf[Axi4]) {
      Axi4TypedDriver(io.s0.asInstanceOf[Axi4], clockDomain)
    } else {
      throw new Exception("simulator driver not implemented")
    }
  }

  // define memory mappings for control registers
  // write address: factory.writeAddress()
  val writeInstruction = Instruction(DualConfig())
  val writeContextId = UInt(config.contextBits bits)
  val (instructionMapping, instructionDocumentation) = if (is64bus) {
    factory.nonStopWrite(writeInstruction, bitOffset = 0)
    factory.nonStopWrite(writeContextId, bitOffset = 32)
    (SizeMapping(base = 4 KiB, size = 4 KiB), "instruction array (64 bits)")
  } else {
    factory.nonStopWrite(writeInstruction, bitOffset = 0)
    writeContextId := factory.writeAddress().resized
    (SizeMapping(base = 8 KiB, size = 4 KiB), "instruction array (32 bits)")
  }
  // read address: factory.readAddress()
  val readoutValue = Bits(factory.busDataWidth bits).assignDontCare()
  val readoutMapping = SizeMapping(base = 128 KiB, size = 128 KiB)
  val readoutDocumentation = "readout array (128 bytes each, 1024 of them at most)"
  factory.readPrimitive(readoutValue, readoutMapping, 0, "readouts")

  // define bus behavior when reading or writing the control registers
  val isWriteHalt = Bool.setAll()
  val isWriteAsk = Bool.clearAll()
  val isWriteDo = Bool.clearAll()
  val isReadHalt = Bool.setAll()
  val isReadAsk = Bool.clearAll()
  val isReadDo = Bool.clearAll()
  def onAskWrite() = {
    isWriteAsk := True
    when(isWriteHalt) {
      factory.writeHalt()
    }
  }
  def onDoWrite() = {
    hardwareInfo.transactionCounter := hardwareInfo.transactionCounter + 1
    isWriteDo := True
  }
  def onAskRead() = {
    isReadAsk := True
    when(isReadHalt) {
      factory.readHalt()
    }
  }
  def onDoRead() = {
    hardwareInfo.transactionCounter := hardwareInfo.transactionCounter + 1
    isReadDo := True
  }
  factory.onWritePrimitive(instructionMapping, haltSensitive = false, instructionDocumentation)(onAskWrite)
  factory.onWritePrimitive(instructionMapping, haltSensitive = true, instructionDocumentation)(onDoWrite)
  factory.onWritePrimitive(readoutMapping, haltSensitive = false, readoutDocumentation)(onAskWrite)
  factory.onWritePrimitive(readoutMapping, haltSensitive = true, readoutDocumentation)(onDoWrite)
  factory.onReadPrimitive(readoutMapping, haltSensitive = false, readoutDocumentation)(onAskRead)
  factory.onReadPrimitive(readoutMapping, haltSensitive = true, readoutDocumentation)(onDoRead)

  // use state machine to handle read and write transactions; read has higher priority because it's blocking operation
  val resizedInstruction = Instruction(config)
  val resizeInstructionHasError = resizedInstruction.resizedFrom(writeInstruction)
  val fsmPushId = pushId.constructReadWriteSyncChannel()
  // data that are used to communicate between states
  val dataWaitFiFoPushLooperInput = Reg(LooperInput(config))
  val fsm = new StateMachine {
    setEncoding(binaryOneHot)

    val stateIdle: State = new State with EntryPoint {
      whenIsActive {
        when(isReadAsk) {
          // check read address and prepare the address into the Mem channel
          goto(stateRead)
        } elsewhen (isWriteAsk) {
          when(instructionMapping.hit(factory.writeAddress())) {
            fsmPushId.readNext(writeContextId)
            goto(stateWriteInstruction)
          } elsewhen (readoutMapping.hit(factory.writeAddress())) {} otherwise {
            hasError := True
            isWriteHalt := False // return immediately
          }
        }
      }
    }

    val stateRead: State = new State {
      whenIsActive {
        readoutValue.clearAll()
        isReadHalt := False
        goto(stateIdle)
      }
    }

    val stateWriteInstruction: State = new State {
      whenIsActive {
        // prepare the looper input
        val looperInput = LooperInput(config)
        looperInput.instruction := resizedInstruction
        if (config.contextBits > 0) { looperInput.contextId := writeContextId }
        looperInput.instructionId := fsmPushId.rData
        looperInput.maximumGrowth.clearAll()
        // prepare the data into the push FIFO
        ccFifoPush.io.push.valid := True
        ccFifoPush.io.push.payload := looperInput
        // check for error
        when(resizeInstructionHasError) { hasError := True }
        // increment push ID
        fsmPushId.writeNext(writeContextId, fsmPushId.rData + 1)
        // update state machine
        when(ccFifoPush.io.push.ready) {
          isWriteHalt := False
          goto(stateIdle)
        } otherwise {
          dataWaitFiFoPushLooperInput := looperInput
          goto(stateWriteWaitFiFoPush)
        }
      }
    }

    val stateWriteWaitFiFoPush: State = new State {
      whenIsActive {
        ccFifoPush.io.push.valid := True
        ccFifoPush.io.push.payload := dataWaitFiFoPushLooperInput
        when(ccFifoPush.io.push.ready) {
          isWriteHalt := False
          goto(stateIdle)
        }
      }
    }
  }

  def simMakePublicSnapshot() = microBlossom.simMakePublicSnapshot()
  def simSnapshot(abbrev: Boolean = true): Json = microBlossom.simSnapshot(abbrev)
  def simPreMatchings(): Seq[DataPreMatching] = microBlossom.simPreMatchings()
}

// sbt 'testOnly *MicroBlossomBusTest'
class MicroBlossomBusTest extends AnyFunSuite {

  test("logic_validity") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    val clockDivideBy = 2

    Config.sim
      .compile(MicroBlossomAxi4(config, clockDivideBy = clockDivideBy))
      // .compile(MicroBlossomAxiLite4(config, clockDivideBy = clockDivideBy))
      // .compile(MicroBlossomAxiLite4Bus32(config, clockDivideBy = clockDivideBy))
      .doSim("logic_validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)
        dut.slow.clockDomain.forkStimulus(period = 10 * clockDivideBy)

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

class MicroBlossomBusGeneratorConf(arguments: Seq[String]) extends ScallopConf(arguments) {
  val graph = opt[String](required = true, descr = "see ./resources/graphs/README.md for more details")
  val outputDir = opt[String](default = Some("gen"), descr = "by default generate the output at ./gen")
  val busType = opt[String](default = Some("Axi4"), descr = s"options: ${MicroBlossomBusType.options.mkString(", ")}")
  val languageHdl = opt[String](default = Some("verilog"), descr = "options: Verilog, VHDL, SystemVerilog")
  val baseAddress = opt[BigInt](default = Some(0), descr = "base address of the memory-mapped module, default to 0")
  // DualConfig
  val broadcastDelay = opt[Int](default = Some(0))
  val convergecastDelay = opt[Int](default = Some(0))
  val contextDepth = opt[Int](default = Some(1), descr = "how many contexts supported")
  val conflictChannels = opt[Int](default = Some(1), descr = "how many conflicts are reported at once")
  val hardCodeWeights = opt[Boolean](default = Some(true), descr = "hard code the edge weights")
  val supportAddDefectVertex = opt[Boolean](default = Some(true), descr = "support AddDefectVertex instruction")
  val supportOffloading = opt[Boolean](default = Some(false), descr = "support offloading optimization")
  val supportLayerFusion = opt[Boolean](default = Some(false), descr = "support layer fusion")
  val injectRegisters =
    opt[List[String]](
      default = Some(List()),
      descr = s"insert register at select stages: ${Stages().stageNames.mkString(", ")}"
    )
  val clockDivideBy = opt[Int](default = Some(2))
  verify()
  def dualConfig = DualConfig(
    filename = graph(),
    broadcastDelay = broadcastDelay(),
    convergecastDelay = convergecastDelay(),
    contextDepth = contextDepth(),
    conflictChannels = conflictChannels(),
    hardCodeWeights = hardCodeWeights(),
    supportAddDefectVertex = supportAddDefectVertex(),
    supportOffloading = supportOffloading(),
    supportLayerFusion = supportLayerFusion(),
    injectRegisters = injectRegisters()
  )
}

// sbt "runMain microblossom.MicroBlossomBusGenerator --help"
// (e.g.) sbt "runMain microblossom.MicroBlossomBusGenerator --graph ./resources/graphs/example_code_capacity_d3.json"
object MicroBlossomBusGenerator extends App {
  val conf = new MicroBlossomBusGeneratorConf(args)
  val dualConfig = conf.dualConfig
  val genConfig = Config.argFolderPath(conf.outputDir())
  // note: deliberately not creating `component` here, otherwise it encounters null pointer error of GlobalData.get()....
  val mode: SpinalMode = conf.languageHdl() match {
    case "verilog" | "Verilog"             => Verilog
    case "VHDL" | "vhdl" | "Vhdl"          => VHDL
    case "SystemVerilog" | "systemverilog" => SystemVerilog
    case _ => throw new Exception(s"HDL language ${conf.languageHdl()} is not recognized")
  }
  genConfig
    .copy(mode = mode)
    .generateVerilog(
      MicroBlossomBusType.generateByName(conf.busType(), dualConfig, conf.clockDivideBy(), conf.baseAddress())
    )
}
