package microblossom.modules

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import spinal.lib.sim._
import microblossom._
import org.scalatest.funsuite.AnyFunSuite

/** Fifo of a fixed depth of 3
  *
  * with mechanism for combinatorial readout;
  *
  * The AXI4 controller needs to run as fast as possible, because it is the bottleneck of CPU-FPGA
  * communication. This buffer is optimized for frequency, with a combinatorial readout that can
  * be used to quickly assert something. I do not develop generic fifo here for simplicity and
  * performance. More generic ones are for future work when this fifo becomes a bottleneck
  */
case class VecFifoDepth4[T <: Data](
    val dataType: HardType[T]
) extends Component {
  val io = new Bundle {
    val push = slave Stream (dataType)
    val pop = master Stream (dataType)
    val forcePush = slave Stream (dataType)
  }

  val values = Vec(Reg(dataType), 4)

  val readHead = Reg(UInt(2 bits)) // 0, 1, 2, 3
  val length = Reg(UInt(2 bits)) // 0, 1, 2, 3

  push.ready := length <= 1 // the force channel
  forcePush.ready := True

  val isEmpty = length === 0

  val writeHead = UInt(2 bits)
  writeHead := writeHead + length

}

// sbt 'testOnly microblossom.modules.VecFifoDepth4Test'
class VecFifoDepth4Test extends AnyFunSuite {

  test("logic_validity") {
    Config.sim
      .compile(VecFifoDepth4(UInt(3 bits)))
      .doSim("logic_validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

        def pushCallback(data: UInt): Boolean = {
          true
        }
        val pushDriver = StreamDriver(dut.io.push, dut.clockDomain)(pushCallback)

      }
  }

}
