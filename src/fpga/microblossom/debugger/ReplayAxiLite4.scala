package microblossom.debugger

import io.circe._
import io.circe.parser.decode
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import scala.io.Source.fromFile
import scala.collection.mutable.ArrayBuffer
import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._

@ConfiguredJsonCodec
case class InterfaceConfig(
    var interface: String,
    var drive: Seq[String],
    var driveWidth: Seq[Int],
    var read: Seq[String],
    var readWidth: Seq[Int]
)
@ConfiguredJsonCodec
case class CycleEntry(
    var drive: Seq[BigInt],
    var read: Seq[BigInt]
)

object InterfaceConfig {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
  val drive =
    List("awvalid", "awaddr", "awprot", "wvalid", "wdata", "wstrb", "bready", "arvalid", "araddr", "arprot", "rready")
  val read = List("awready", "wready", "bvalid", "bresp", "arready", "rvalid", "rdata", "rresp")
}
object CycleEntry {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

case class DriveBundle(interfaceConfig: InterfaceConfig) extends Bundle {
  val values = ArrayBuffer[UInt]()
  for ((name, index) <- InterfaceConfig.drive.zipWithIndex) {
    val value = UInt(interfaceConfig.driveWidth(index) bits)
    valCallbackRec(value, name)
    values.append(value)
  }
  def value(name: String): UInt = {
    values(InterfaceConfig.drive.indexOf(name))
  }
}

case class ReadBundle(interfaceConfig: InterfaceConfig) extends Bundle {
  val values = ArrayBuffer[UInt]()
  for ((name, index) <- InterfaceConfig.read.zipWithIndex) {
    val value = UInt(interfaceConfig.readWidth(index) bits)
    valCallbackRec(value, name)
    values.append(value)
  }
  def value(name: String): UInt = {
    values(InterfaceConfig.read.indexOf(name))
  }
}

case class DebuggerFileAxiLite4(filePath: String) {
  val lines = fromFile(filePath).getLines
  var interfaceConfig: InterfaceConfig = null
  val cycleEntries = ArrayBuffer[CycleEntry]()
  for ((line, lineIndex) <- lines.zipWithIndex) {
    if (lineIndex == 0) {
      interfaceConfig = decode[InterfaceConfig](line) match {
        case Right(interfaceConfig) => interfaceConfig
        case Left(ex)               => throw ex
      }
      assert(interfaceConfig.interface == "AxiLite4")
      assert(interfaceConfig.drive == InterfaceConfig.drive)
      assert(interfaceConfig.read == InterfaceConfig.read)
      assert(interfaceConfig.driveWidth.length == InterfaceConfig.drive.length)
      assert(interfaceConfig.readWidth.length == InterfaceConfig.read.length)
    } else {
      val cycleEntry = decode[CycleEntry](line) match {
        case Right(cycleEntry) => cycleEntry
        case Left(ex)          => throw ex
      }
      assert(cycleEntry.drive.length == InterfaceConfig.drive.length)
      assert(cycleEntry.read.length == InterfaceConfig.read.length)
      cycleEntries.append(cycleEntry)
    }
  }
  def length = cycleEntries.length
  def driveIterator = {
    cycleEntries.iterator
      .map((cycleEntry) => {
        val drive = DriveBundle(interfaceConfig)
        for ((name, index) <- InterfaceConfig.drive.zipWithIndex) {
          drive.values(index) := U(cycleEntry.drive(index))
        }
        drive
      })
  }
  def readIterator = {
    cycleEntries.iterator
      .map((cycleEntry) => {
        val read = ReadBundle(interfaceConfig)
        for ((name, index) <- InterfaceConfig.read.zipWithIndex) {
          read.values(index) := U(cycleEntry.read(index))
        }
        read
      })
  }
}

case class ReplayAxiLite4(
    debuggerFile: DebuggerFileAxiLite4,
    dualConfig: DualConfig,
    clockDivideBy: Int = 1, // divided clock at io.dividedClock; note the clock must be synchronous and 0 phase aligned
    baseAddress: BigInt = 0
) extends Component {
  val indexBits = log2Up(debuggerFile.length)
  val interfaceConfig = debuggerFile.interfaceConfig

  val io = new Bundle {
    val index = out(UInt(indexBits bits))
    val drive = out(DriveBundle(interfaceConfig))
    drive.setName("s0") // avoid adding the `io_drive_` prefix in the signal generated
    val readExpected = out(ReadBundle(interfaceConfig))
    readExpected.setName("s0")
    // val error = out(Bool())
    // val finished = out(Bool())
  }

  val driveMem = Mem(DriveBundle(interfaceConfig), debuggerFile.length) init (debuggerFile.driveIterator.toSeq)
  val readMem = Mem(ReadBundle(interfaceConfig), debuggerFile.length) init (debuggerFile.readIterator.toSeq)

  val counter = Counter(indexBits bits)
  io.index := counter.value

  when(counter.value =/= debuggerFile.length - 1) {
    counter.increment()
  }

  val driveValue = driveMem.readSync(counter.valueNext)
  io.drive := driveValue

  val readValue = readMem.readSync(counter.valueNext)
  io.readExpected := readValue

  // val microBlossom = MicroBlossomAxiLite4.generate(config, conf.clockDivideBy(), conf.baseAddress())
}

// (e.g.) sbt "runMain microblossom.debugger.ReplayAxiLite4Generator ./simWorkspace/MicroBlossomHost/test_micro_blossom/s0.debugger --graph ./resources/graphs/example_code_capacity_d3.json"
object ReplayAxiLite4Generator extends App {
  def getParameters(args: Array[String]) = {
    if (args.length < 1) {
      Console.err.println("usage: <debugger_path> <...microblossom configuration>")
      sys.exit(1)
    }
    val debuggerPath = args(0)
    val debuggerFile = DebuggerFileAxiLite4(debuggerPath)
    val conf = new MicroBlossomGeneratorConf(args.tail)
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
    val mode: SpinalMode = conf.languageHdl() match {
      case "verilog" | "Verilog"             => Verilog
      case "VHDL" | "vhdl" | "Vhdl"          => VHDL
      case "SystemVerilog" | "systemverilog" => SystemVerilog
      case _ => throw new Exception(s"HDL language ${conf.languageHdl()} is not recognized")
    }
    (genConfig, mode, config, conf, debuggerFile)
  }

  val (genConfig, mode, config, conf, debuggerFile) = getParameters(args)
  genConfig
    .copy(mode = mode)
    .generateVerilog(
      ReplayAxiLite4(debuggerFile, config, conf.clockDivideBy(), conf.baseAddress())
    )
}

// (e.g.) sbt "runMain microblossom.debugger.ReplayAxiLite4Test ./simWorkspace/MicroBlossomHost/test_micro_blossom/s0.debugger --graph ./resources/graphs/example_code_capacity_d3.json"
object ReplayAxiLite4Test extends App {
  val (genConfig, mode, config, conf, debuggerFile) = ReplayAxiLite4Generator.getParameters(args)
  Config.sim
    .compile(ReplayAxiLite4(debuggerFile, config, conf.clockDivideBy(), conf.baseAddress()))
    .doSim("logic validity") { dut =>
      dut.clockDomain.forkStimulus(period = 10)

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
    }

}
