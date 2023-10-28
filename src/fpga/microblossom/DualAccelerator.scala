package microblossom

import spinal.core._
import spinal.lib._
import util._

object DualAcceleratorState extends SpinalEnum {
  val Normal, Busy, InstructionError = newElement()
}

case class DualAccelerator(config: DualConfig) extends Component {
  val io = new Bundle {
    val instruction = in(Instruction())
    val state = out(DualAcceleratorState())
  }

  val instructionReg = RegNext(io.instruction)

  io.state := DualAcceleratorState.Normal

  val broadcastMessage = Instruction(config)

  broadcastMessage.connect(io.instruction)
  val broadcastInstruction = Delay(RegNext(broadcastMessage), config.broadcastDelay)

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
