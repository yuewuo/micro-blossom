package microblossom

import spinal.core._

case class DualConfig(
    VertexBits: Int,
    WeightBits: Int
) {
  val InstructionBits = 2 * VertexBits + 2

  def sanity_check(): Unit = {
    assert(WeightBits + 2 < InstructionBits)
  }
}

object DualAccelerator extends SpinalEnum {
  val Normal = newElement()
}

// case class DualAccelerator() extends Component {
//   val io = new Bundle {
//     val
//     val state = out DualAccelerator()
//   }
// }
