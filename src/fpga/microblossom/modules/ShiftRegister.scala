package microblossom.modules

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._

case class ShiftRegister[T <: Data](dataType: HardType[T], depth: Int, initFunc: T => Unit) extends Component {
  val io = new Bundle {
    val input = in(dataType())
    val output = out(dataType())
  }

  if (depth > 0) {

    val registers = Vec.fill(depth)(Reg(dataType()))
    registers.foreach(initFunc)

    for (i <- 1 until depth) {
      registers(i) := registers(i - 1)
    }

    registers(0) := io.input
    io.output := registers(depth - 1)

  } else {
    io.output := io.input
  }
}
