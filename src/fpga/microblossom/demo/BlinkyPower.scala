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

class BlinkyPower extends Component {
  val io = new Bundle {
    val leds = out Bits (6 bits)
  }

  val vexRiscVPlugins = ArrayBuffer(
    new PcManagerSimplePlugin(
      resetVector = 0x00000000L,
      relaxedPcCalculation = false
    ),
    // new IBusSimplePlugin(
    //   resetVector = 0x00000000L,
    //   cmdForkOnSecondStage = true,
    //   cmdForkPersistence = true
    // ),
    new IBusCachedPlugin(
      resetVector = 0x00000000L,
      prediction = DYNAMIC_TARGET, // FullMaxPerf(DYNAMIC_TARGET) vs Briey(STATIC)
      compressedGen = true, // Add RV32C ISA Extension
      injectorStage = true, // needed for RV32C, see https://github.com/SpinalHDL/VexRiscv/issues/93
      historyRamSizeLog2 = 8,
      config = InstructionCacheConfig(
        cacheSize = 4096 * 2,
        bytePerLine = 32,
        wayCount = 1,
        addressWidth = 32,
        cpuDataWidth = 32,
        memDataWidth = 32,
        catchIllegalAccess = true,
        catchAccessFault = true,
        asyncTagMemory = false,
        twoCycleRam = true, // FullMaxPerf(false) vs Briey(true)
        twoCycleCache = true
      )
    ),
    // new DBusSimplePlugin(
    //   catchAddressMisaligned = false,
    //   catchAccessFault = false
    // ),
    new DBusCachedPlugin(
      config = new DataCacheConfig(
        cacheSize = 4096 * 2,
        bytePerLine = 32,
        wayCount = 1,
        addressWidth = 32,
        cpuDataWidth = 32,
        memDataWidth = 32,
        catchAccessError = true,
        catchIllegal = true,
        catchUnaligned = true
        // withLrSc = true, // part of the RV32A extension, used by spin::Mutex
        // withAmo = true
      ),
      memoryTranslatorPortConfig = null
    ),
    new StaticMemoryTranslatorPlugin( // required for cached IBus and dBus
      ioRange = _(31 downto 28) === 0xf
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
      bypassExecute = true,
      bypassMemory = true,
      bypassWriteBack = true,
      bypassWriteBackBuffer = true,
      pessimisticUseSrc = false,
      pessimisticWriteRegFile = false,
      pessimisticAddressMatch = false
    ),
    new BranchPlugin(
      earlyBranch = false,
      catchAddressMisaligned = false
    ),
    // these two plugins are required because Rust only have RV32I and RV32IMAC support; I need Atomic extension so
    // it has to add the M and C extensions to make it work. However, the binary doesn't necessary use them
    new MulPlugin,
    // new DivPlugin,  // try to avoid using any integer divisions in the code (there shouldn't be any...)

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
      case plugin: IBusCachedPlugin => iBus = plugin.iBus.toAxi4ReadOnly()
      case plugin: DBusSimplePlugin => dBus = plugin.dBus.toAxi4Shared()
      case plugin: DBusCachedPlugin => dBus = plugin.dBus.toAxi4Shared(true)
      case plugin: CsrPlugin => {
        plugin.externalInterrupt := False
        plugin.timerInterrupt := False
      }
      case _ =>
    }

    val ram = Axi4SharedOnChipRam(
      dataWidth = 32,
      byteCount = 256 kB,
      idWidth = 4
    )

    val mockDualAccelerator = Axi4SharedOnChipRam(
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
      ram.io.axi -> (0x00000000L, ram.byteCount),
      mockDualAccelerator.io.axi -> (0xf1000000L, 4 kB),
      apbBridge.io.axi -> (0xf0000000L, 1 MB)
    )

    axiCrossbar.addConnections(
      iBus -> List(ram.io.axi),
      dBus -> List(ram.io.axi, apbBridge.io.axi, mockDualAccelerator.io.axi)
    )

    axiCrossbar.build()

    val ledReg = Apb3SlaveFactory(apbBridge.io.apb)
      .createReadWrite(Bits(6 bits), 0xf0000000L, 0)
    io.leds := ledReg
  }

}

// sbt "runMain microblossom.demo.BlinkyPowerVerilog"
object BlinkyPowerVerilog extends App {
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

  def buildTop(): BlinkyPower = {
    val top = new BlinkyPower()
    val program =
      loadProgram(
        // "src/cpu/embedded/target/riscv32i-unknown-none-elf/release/embedded_blossom.bin",
        "src/cpu/embedded/target/riscv32imac-unknown-none-elf/release/embedded_blossom.bin",
        top.core.ram.ram.byteCount / 4 // how many words (four-byte)
      )
    top.core.ram.ram.initBigInt(program)
    top
  }

  Config.spinal.generateVerilog(buildTop())
}

// sbt "runMain microblossom.demo.BlinkyPowerTestA" && gtkwave simWorkspace/BlinkyPower/testA.fst
object BlinkyPowerTestA extends App {
  Config.sim.compile(BlinkyPowerVerilog.buildTop()).doSim("testA") { dut =>
    dut.externalClockDomain.forkStimulus(10)
    sleep(2000000)
  }
}
