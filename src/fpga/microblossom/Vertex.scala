package microblossom

import spinal.core._
import microblossom._

case class VertexOutput(config: DualConfig) extends Bundle {
  // fetch stage
  val speed = Bits(2 bits)
  // execute stage

  // update stage

  // write stage
}

case class Vertex2() extends Component {
  val io = new Bundle {
    val opcode = out(Bits(2 bits))
  }
}

case class Vertex(config: DualConfig, vertexIndex: Int) extends Component {
  // printf("hello\n");
  val io = new Bundle {
    val instruction = in(InternalInstruction(config))
    val opcode = out(Bits(2 bits))
    // val vertexOutputs = out(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(VertexOutput(config)))
    // val edgeInputs = in(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(EdgeOutput(config)))
  }

  io.opcode := io.instruction.opcode

  // there are 4 stages: fetch, execute, update, write

}
