package microblossom

/*
 * # 64 bit timer
 *
 * This module provides an alternative for the implementation of `get_native_time`, which
 * is 32 bit and too naive. Also, this module helps me to learn how to create an AXI4-compatible
 * package, so that more complex designs could follow.
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

case class Axi4Timer(baseAddress: BigInt = 0, axi4Config: Axi4Config = VersalAxi4Config()) extends Component {
  val io = new Bundle {
    val s0 = slave(
      Axi4(axi4Config)
    )
  }

  val factory = Axi4SlaveFactory(io.s0)

  val counter = Reg(UInt(64 bits)) init 0
  counter := counter + 1

  factory.read(counter, baseAddress)

  Axi4SpecRenamer(io.s0) // to follow the naming convention
}

// sbt 'testOnly *Axi4TimerTest'
class Axi4TimerTest extends AnyFunSuite {

  test("construct an Axi4Timer Module") {
    Config.spinal().generateVerilog(Axi4Timer())
  }

  test("logic_validity") {
    Config.sim
      .compile(Axi4Timer())
      .doSim("logic_validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

      // val driver = AxiLite4Driver(dut.io.bus, dut.clockDomain)
      // driver.reset()

      // val version = driver.read(0)
      // printf("version: %x\n", version)
      }

  }

}

// sbt "runMain Axi4TimerDirect <folder>"
// create an AXI4 interface that can be directly plugged in with Versal FPD AXI4
object Axi4TimerDirect extends App {
  if (args.length != 1) {
    Console.err.println("usage: <folder>")
    sys.exit(1)
  }
  // [option 1] use full 44 bit address, but have to hardcode with base address, which is 0xA400_0000
  // Config.argFolderPath(args(0)).generateVerilog(Axi4Timer(BigInt("A4000000", 16)))
  // [option 2 (preferred)] use only 28 bit address, don't have to hardcode the base address anymore in the Verilog
  Config.argFolderPath(args(0)).generateVerilog(Axi4Timer(axi4Config = VersalAxi4Config(addressWidth = 28))) // 256MB
}

// sbt "runMain Axi4TimerMinimal <folder>"
object Axi4TimerMinimal extends App {
  if (args.length != 1) {
    Console.err.println("usage: <folder>")
    sys.exit(1)
  }
  Config
    .argFolderPath(args(0))
    .generateVerilog(
      Axi4Timer(
        baseAddress = 0,
        axi4Config = MinimalAxi4Config(addressWidth = 3) // 8 bytes
      )
    )
}
