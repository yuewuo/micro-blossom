package microblossom

import spinal.core._
import spinal.lib._
import util._

object DualAcceleratorState extends SpinalEnum {
  val Normal, Busy, InstructionError = newElement()
}

case class InternalInstruction(config: DualConfig) extends Bundle {
  val raw = Bits(config.instructionBits bits)

  def connect(instruction: Bits): Unit = {
    assert(instruction.getWidth == 32) // the instruction must be 32 bits
    val opcode_1 = instruction(1 downto 0)
    raw(1 downto 0) := opcode_1
    switch(opcode_1) {
      is(Architecture.OpCode1.SetSpeed) {
        raw(1 + config.vertexBits downto 2) := instruction(1 + config.vertexBits downto 2)
        raw(1 + 2 * config.vertexBits downto 2 + config.vertexBits) := instruction(
          16 + config.vertexBits downto 17
        )
      }
      default {
        raw(1 + 2 * config.vertexBits downto 2) := B"0".resized
      }
    }
  }

  def opcode = raw(1 downto 0)
}

case class DualAccelerator(config: DualConfig) extends Component {
  val io = new Bundle {
    val instruction = in(Bits(32 bits))
    val state = out(DualAcceleratorState())
    val internalInstruction = out(InternalInstruction(config))
  }

  val instructionReg = RegNext(io.instruction)

  io.state := DualAcceleratorState.Normal
  io.internalInstruction.connect(io.instruction)
  val internalInstructionReg = RegNext(io.internalInstruction)
  val broadcastInstruction = Delay(internalInstructionReg, config.broadcastDelay)

  // instantiate vertices and edges
  val vertices = Seq
    .range(0, config.vertexNum)
    .map(vertexIndex => new Vertex(config, vertexIndex))

  vertices.foreach(vertex => {
    vertex.io.instruction := broadcastInstruction
  })

  val edges = Seq
    .range(0, config.edgeNum)
    .map(edgeIndex => new Edge(config, edgeIndex))

  edges.foreach(edge => {
    edge.io.instruction := broadcastInstruction
  })

  // connect the vertices and edges
  for (vertexIndex <- Range(0, config.vertexNum)) {
    val vertex = vertices(vertexIndex)
    for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
      val edge = edges(edgeIndex)
      val localIndexOfVertex = config.localIndexOfVertex(edgeIndex, vertexIndex)
      val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
      // vertex.io.vertexOutputs(localIndexOfEdge) <> edge.io.vertexInputs(localIndexOfVertex)
      // vertex.io.edgeInputs(localIndexOfEdge) <> edge.io.edgeOutputs(localIndexOfVertex)
    }
  }

  // TODO: gather the results in a tree structure. tip: use reduceBalancedTree function
  // https://spinalhdl.github.io/SpinalDoc-RTD/master/SpinalHDL/Data%20types/Vec.html#lib-helper-functions
}
