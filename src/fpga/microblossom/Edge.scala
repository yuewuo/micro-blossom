package microblossom

import spinal.core._
import microblossom._

case class EdgeOutput(config: DualConfig) extends Bundle {
  val someInfoToVertex = Bits(3 bits)
}

case class Edge(config: DualConfig, edgeIndex: Int) extends Component {
  val io = new Bundle {
    val instruction = in(Instruction(config))
    val opCode = out(Bits(2 bits))
    val edgeOutputs = out(Vec.fill(2)(EdgeOutput(config)))
    val vertexInputs = in(Vec.fill(2)(VertexOutput(config)))
  }

  io.opCode := io.instruction.opCode
}
