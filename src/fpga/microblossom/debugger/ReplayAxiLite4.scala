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

case class TestSignals() extends Bundle {
  val rvalid = Bool()
  val rdata = Bool()
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
    val readActual = out(ReadBundle(interfaceConfig))
    readActual.setName("ra")
    val tests = out(TestSignals())
    val error = out(Bool())
    val finished = out(Bool())
  }

  val driveMem = Mem(DriveBundle(interfaceConfig), debuggerFile.length) init (debuggerFile.driveIterator.toSeq)
  val readMem = Mem(ReadBundle(interfaceConfig), debuggerFile.length) init (debuggerFile.readIterator.toSeq)

  val counter = Counter(indexBits bits)
  io.index := counter.value - 1

  when(counter.value =/= debuggerFile.length - 1) {
    io.finished := False
    counter.increment()
  } otherwise {
    io.finished := True
  }

  val driveValue = driveMem.readSync(counter.value)
  io.drive := driveValue

  val readValue = readMem.readSync(counter.value)
  io.readExpected := readValue

  val microBlossom = MicroBlossomAxiLite4(dualConfig, clockDivideBy, baseAddress)
  // drive: "awvalid", "awaddr", "awprot", "wvalid", "wdata", "wstrb", "bready", "arvalid", "araddr", "arprot", "rready"
  microBlossom.io.s0.aw.valid := driveValue.value("awvalid").asBool
  microBlossom.io.s0.aw.addr := driveValue.value("awaddr")
  microBlossom.io.s0.aw.prot := driveValue.value("awprot").asBits
  microBlossom.io.s0.w.valid := driveValue.value("wvalid").asBool
  microBlossom.io.s0.w.data := driveValue.value("wdata").asBits
  microBlossom.io.s0.w.strb := driveValue.value("wstrb").asBits
  microBlossom.io.s0.b.ready := driveValue.value("bready").asBool
  microBlossom.io.s0.ar.valid := driveValue.value("arvalid").asBool
  microBlossom.io.s0.ar.addr := driveValue.value("araddr")
  microBlossom.io.s0.ar.prot := driveValue.value("arprot").asBits
  microBlossom.io.s0.r.ready := driveValue.value("rready").asBool
  // read: "awready", "wready", "bvalid", "bresp", "arready", "rvalid", "rdata", "rresp"
  io.readActual.value("awready") := microBlossom.io.s0.aw.ready.asUInt
  io.readActual.value("wready") := microBlossom.io.s0.w.ready.asUInt
  io.readActual.value("bvalid") := microBlossom.io.s0.b.valid.asUInt
  io.readActual.value("bresp") := microBlossom.io.s0.b.resp.asUInt
  io.readActual.value("arready") := microBlossom.io.s0.ar.ready.asUInt
  io.readActual.value("rvalid") := microBlossom.io.s0.r.valid.asUInt
  io.readActual.value("rdata") := microBlossom.io.s0.r.data.asUInt
  io.readActual.value("rresp") := microBlossom.io.s0.r.resp.asUInt

  io.tests.rvalid := io.readExpected.value("rvalid") === io.readActual.value("rvalid")
  io.tests.rdata := !io.readActual.value("rvalid").asBool || (
    io.readExpected.value("rdata") === io.readActual.value("rdata")
  )

  io.error := !io.tests.rvalid || !io.tests.rdata

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
    val config = conf.dualConfig
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
    .doSim("replay") { dut =>
      dut.clockDomain.forkStimulus(period = 10)

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
      for (idx <- 0 to debuggerFile.length) { dut.clockDomain.waitSampling() }
      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
    }

}
