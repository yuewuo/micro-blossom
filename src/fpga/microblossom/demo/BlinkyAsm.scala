package microblossom.demo

import microblossom.Config
import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba3.apb._
import spinal.lib.bus.amba4.axi._
import spinal.core.sim._
import vexriscv.plugin._
import vexriscv._
import vexriscv.ip.{DataCacheConfig, InstructionCacheConfig}
import scala.collection.mutable.ArrayBuffer
import java.nio.file.{Files, Paths}
import java.math.BigInteger

class BlinkyAsm extends Component {
  val io = new Bundle {
    val leds = out Bits (6 bits)
  }

  val vexRiscVPlugins = ArrayBuffer(
    new PcManagerSimplePlugin(0x00000000L, false),
    new IBusSimplePlugin(
      resetVector = 0x00000000L,
      cmdForkOnSecondStage = true,
      cmdForkPersistence = true
    ),
    new DBusSimplePlugin(
      catchAddressMisaligned = false,
      catchAccessFault = false
    ),
    new DecoderSimplePlugin(
      catchIllegalInstruction = true // Rust relies on illegal instruction catch
    ),
    new RegFilePlugin(
      regFileReadyKind = plugin.SYNC,
      zeroBoot = true
    ),
    new IntAluPlugin,
    new SrcPlugin(
      separatedAddSub = false,
      executeInsertion = false
    ),
    new LightShifterPlugin,
    new HazardSimplePlugin(
      bypassExecute = false,
      bypassMemory = false,
      bypassWriteBack = false,
      bypassWriteBackBuffer = false
    ),
    new BranchPlugin(
      earlyBranch = false,
      catchAddressMisaligned = false
    ),
    new CsrPlugin(
      config = CsrPluginConfig(
        catchIllegalAccess = false,
        mvendorid = null,
        marchid = null,
        mimpid = null,
        mhartid = null,
        misaExtensionsInit = 66,
        misaAccess = CsrAccess.NONE,
        mtvecAccess = CsrAccess.NONE,
        mtvecInit = 0x80000020L,
        mepcAccess = CsrAccess.READ_WRITE,
        mscratchGen = false,
        mcauseAccess = CsrAccess.READ_ONLY,
        mbadaddrAccess = CsrAccess.READ_WRITE,
        mcycleAccess = CsrAccess.NONE,
        minstretAccess = CsrAccess.NONE,
        ecallGen = false,
        wfiGenAsWait = false,
        ucycleAccess = CsrAccess.NONE,
        ebreakGen = true // Rust relies on software ebreak instruction support
      )
    )
  )

  val externalClockDomain = ClockDomain.external(
    "io", // result in a clock named "io_clk"
    ClockDomainConfig(resetKind = BOOT) // does not generate a reset IO
  )

  // AXI spec requires a long reset
  val resetCtrl = new ClockingArea(externalClockDomain) {
    val systemReset = RegInit(True)
    val resetCounter = RegInit(U"6'h0")
    when(resetCounter =/= 63) {
      resetCounter := resetCounter + 1
    } otherwise {
      systemReset := False
    }
  }

  val coreDomain = ClockDomain(
    clock = externalClockDomain.readClockWire,
    reset = resetCtrl.systemReset
  )

  val core = new ClockingArea(coreDomain) {
    val vexRiscVConfig = VexRiscvConfig(plugins = vexRiscVPlugins)
    val cpu = new VexRiscv(vexRiscVConfig)
    var iBus: Axi4ReadOnly = null
    var dBus: Axi4Shared = null
    for (plugin <- vexRiscVConfig.plugins) plugin match {
      case plugin: IBusSimplePlugin => iBus = plugin.iBus.toAxi4ReadOnly()
      case plugin: DBusSimplePlugin => dBus = plugin.dBus.toAxi4Shared()
      case plugin: CsrPlugin => {
        plugin.externalInterrupt := False
        plugin.timerInterrupt := False
      }
      case _ =>
    }

    val ram = Axi4SharedOnChipRam(
      dataWidth = 32,
      byteCount = 4 kB,
      idWidth = 4
    )

    val apbBridge = Axi4SharedToApb3Bridge(
      addressWidth = 32,
      dataWidth = 32,
      idWidth = 0
    )

    val axiCrossbar = Axi4CrossbarFactory()
    axiCrossbar.addSlaves(
      ram.io.axi -> (0x00000000L, 4 kB),
      apbBridge.io.axi -> (0xf0000000L, 1 MB)
    )

    axiCrossbar.addConnections(
      iBus -> List(ram.io.axi),
      dBus -> List(ram.io.axi, apbBridge.io.axi)
    )

    axiCrossbar.build()

    val ledReg = Apb3SlaveFactory(apbBridge.io.apb)
      .createReadWrite(Bits(6 bits), 0xf0000000L, 0)
    io.leds := ledReg
  }

}

// sbt "runMain microblossom.demo.BlinkyAsmVerilog"
object BlinkyAsmVerilog extends App {
  def loadProgram(path: String, padToWordCount: Int): Array[BigInt] = {
    Files
      .readAllBytes(Paths.get(path))
      .grouped(4)
      .map(wordBytes => {
        BigInt(new BigInteger(wordBytes.padTo(8, 0.toByte).reverse.toArray))
      })
      .padTo(padToWordCount, BigInt(0))
      .toArray
  }

  def buildTop(): BlinkyAsm = {
    val top = new BlinkyAsm()
    // val program = loadProgram("src/fpga/microblossom/demo/empty.bin", 1024)
    // val program = loadProgram("src/fpga/microblossom/demo/blink.bin", 1024)
    val program = loadProgram("src/cpu/embedded/target/riscv32i-unknown-none-elf/release/embedded_blossom.bin", 1024)
    top.core.ram.ram.initBigInt(program)
    top
  }

  Config.spinal().generateVerilog(buildTop())
}

// sbt "runMain microblossom.demo.BlinkyAsmTestA" && gtkwave simWorkspace/BlinkyAsm/testA.fst
object BlinkyAsmTestA extends App {
  Config.sim.compile(BlinkyAsmVerilog.buildTop()).doSim("testA") { dut =>
    dut.externalClockDomain.forkStimulus(10)
    sleep(100000)
  }
}
