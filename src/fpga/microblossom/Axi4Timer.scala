/*
 * # 64 bit timer
 *
 * This module provides an alternative for the implementation of `get_native_time`, which
 * is 32 bit and too naive. Also, this module helps me to learn how to create an AXI4-compatible
 * package, so that more complex designs could follow.
 *
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

case class Axi4Timer() extends Component {
  val io = new Bundle {
    val s0 = slave(
      Axi4(VersalAxi4Config())
    )
  }

  val factory = Axi4SlaveFactory(io.s0)

  val counter = Reg(UInt(64 bits)) init 0
  counter := counter + 1

  factory.read(counter, 0)

  Axi4SpecRenamer(io.s0) // to follow the naming convention
}

// sbt 'testOnly *Axi4TimerTest'
class Axi4TimerTest extends AnyFunSuite {

  test("construct an Axi4Timer Module") {
    Config.spinal().generateVerilog(Axi4Timer())
  }

  test("logic validity") {
    Config.sim
      .compile(Axi4Timer())
      .doSim("logic validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

      // val driver = AxiLite4Driver(dut.io.bus, dut.clockDomain)
      // driver.reset()

      // val version = driver.read(0)
      // printf("version: %x\n", version)
      }

  }

}

// sbt "runMain Axi4TimerGenerate <folder>"
object Axi4TimerGenerate extends App {
  if (args.length != 1) {
    Console.err.println("usage: <folder>")
    sys.exit(1)
  }
  Config.argFolderPath(args(0)).generateVerilog(Axi4Timer())
}
