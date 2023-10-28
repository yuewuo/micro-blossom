package microblossom.learn

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite
import microblossom._

case class LearnMem() extends Component {
  val io = new Bundle {
    val read = in Bool ()
    val readAddress = in UInt (10 bits)
    val readResult = out Bits (36 bits)
    val write = in Bool ()
    val writeAddress = in UInt (10 bits)
    val writeValue = in Bits (36 bits)
  }

  val ram = Mem(Bits(36 bits), 1024)

  io.readResult := ram.readSync(
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

  test("memory write") {
    Config.spinal.generateVerilog(LearnMem())
  }

}
