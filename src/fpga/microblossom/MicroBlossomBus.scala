package microblossom

/*
 * # Micro Blossom Accelerator
 *
 * This module provides unified access to the Distributed Dual module with AXI4 interface.
 *
 *
 * Note:
 *     1. Always set maximumGrowth to 0 before executing the commands, otherwise there might be data races
 *       (It happens because writing command only checks for executeLatency cycles of data race, however, the primal
 *        offloaded grow unit may issue command when the data comes back)
 *        When designing drivers, set maximumGrowth only when you are trying to read obstacles and set it to 0 once done
 *        to make sure there is no spontaneous grow
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
//  - (64 bits only) the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (WO) (32 bits instruction, 16 bits context id)
//    0x1008: (WO) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
//  - (32 bits only) the following 4KB section is designed for 32 bit bus where context id is encoded in the address
//    0x2000: 32 bits instruction for context 0
//    0x2004: 32 bits instruction for context 1
//    0x2008: ... repeat for 1024: in total 4KB space
// 2. 128KB context readouts at [0x2_0000, 0x4_0000), each context takes 128 byte space, assuming no more than 1024 contexts
//    [context 0]
//      0: (RW) 16 bits maximum growth (offloaded primal), when 0, disable offloaded primal,
//                  write to this field will automatically clear accumulated grown value
//      2: (RW) 16 bits accumulated grown value (for primal offloading)
//      4: (RO) 16 bits growable value (writing to this position has no effect)
//      8: (RW) 64 bits timestamp of receiving the last ``load obstacles'' instruction
//      16: (RW) 64 bits timestamp of receiving the last ``growable = infinity'' response
//      (at most 6 concurrent conflict report, large enough)
//      32: (RO) 128 bits conflict value [0] (96 bits conflict value, 8 bits is_valid)
//      48: (RO) 128 bits conflict value [1]
//      64: (RO) 128 bits conflict value [2]
//         ...
//    [context 1]
//      128: (RO) 32 bits growable value, when 0, the conflict values are valid
//         ...
//

case class MicroBlossomBus[T <: IMasterSlave, F <: BusSlaveFactoryDelayed](
    dualConfig: DualConfig,
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
      U(dualConfig.contextDepth, 32 bits) ## U(DualConfig.version, 32 bits),
      address = 8,
      documentation = "micro-blossom version and context depth"
    )
    factory.readMultiWord(
      U(dualConfig.weightBits, 8 bits) ## U(dualConfig.vertexBits, 8 bits) ## U(dualConfig.conflictChannels, 8 bits),
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

  // instantiate Micro Blossom module
  val ioConfig = DualConfig()
  ioConfig.contextDepth = dualConfig.contextDepth
  ioConfig.weightBits = dualConfig.weightBits
  ioConfig.vertexBits = dualConfig.vertexBits

  val micro = new Area {
    val io = new Bundle {
      // only inputs need to be registered
      val message = Reg(BroadcastMessage(ioConfig, explicitReset = false))
      message.addTag(crossClockDomain)
      // outputs are slow anyway
      val maxGrowable = ConvergecastMaxGrowable(ioConfig.weightBits)
      maxGrowable.addTag(crossClockDomain)
      val conflict = ConvergecastConflict(ioConfig.vertexBits)
      conflict.addTag(crossClockDomain)
    }

    val clockDomain = ClockDomain(
      clock = slowClk,
      reset = ClockDomain.current.readResetWire,
      config = ClockDomainConfig(
        clockEdge = RISING,
        resetKind = SYNC,
        resetActiveLevel = HIGH
      )
    )

    val clockingArea = new ClockingArea(clockDomain) {
      val microBlossom = MicroBlossomMocker(dualConfig, ioConfig)
      microBlossom.io.message := io.message
      io.maxGrowable := microBlossom.io.maxGrowable
      io.conflict := microBlossom.io.conflict
    }
  }

  def getSimDriver(): TypedDriver = {
    if (io.s0.isInstanceOf[AxiLite4]) {
      AxiLite4TypedDriver(io.s0.asInstanceOf[AxiLite4], clockDomain)
    } else if (io.s0.isInstanceOf[Axi4]) {
      Axi4TypedDriver(io.s0.asInstanceOf[Axi4], clockDomain)
    } else {
      throw new Exception("simulator driver not implemented")
    }
  }
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
        dut.micro.clockDomain.forkStimulus(period = 10 * clockDivideBy)

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
  val config = conf.dualConfig
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
      MicroBlossomBusType.generateByName(conf.busType(), config, conf.clockDivideBy(), conf.baseAddress())
    )
}
