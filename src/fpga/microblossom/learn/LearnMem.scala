package microblossom.learn

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite
import microblossom._
import cats.instances.either

case class LearnMem() extends Component {
  val io = new Bundle {
    val read = in Bool ()
    val readAddress = in UInt (10 bits)
    val readValue = out Bits (31 bits)
    val write = in Bool ()
    val writeAddress = in UInt (10 bits)
    val writeValue = in Bits (31 bits)
  }

  val ram = Mem(Bits(31 bits), 1024)

  io.readValue := ram.readSync(
    address = io.readAddress,
    enable = io.read
  )

  ram.write(
    address = io.writeAddress,
    data = io.writeValue,
    enable = io.write
  )

}

// sbt 'testOnly *LearnMemTest'
class LearnMemTest extends AnyFunSuite {

  test("generate verilog") {
    Config.spinal.generateVerilog(LearnMem())
  }

  test("behavior test") {
    // gtkwave simWorkspace/LearnMem/testA.fst
    Config.sim.compile(LearnMem()).doSim("testA") { dut =>
      dut.clockDomain.forkStimulus(period = 10)

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

      dut.clockDomain.waitSampling()
      dut.io.write #= true
      dut.io.writeAddress #= 0
      dut.io.writeValue #= 0x1234

      dut.clockDomain.waitSampling()
      dut.io.write #= true
      dut.io.writeAddress #= 0x33
      dut.io.writeValue #= 0x5678
      dut.io.read #= true
      dut.io.readAddress #= 0

      dut.clockDomain.waitSampling()
      dut.io.write #= false
      dut.io.readAddress #= 0x33
      sleep(1)
      assert(dut.io.readValue.toInt == 0x1234)

      dut.clockDomain.waitSampling()
      dut.io.read #= false
      sleep(1)
      assert(dut.io.readValue.toInt == 0x5678)

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
    }
  }

}
