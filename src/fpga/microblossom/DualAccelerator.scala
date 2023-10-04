package microblossom

import spinal.core._
import util._
import io.circe.parser.decode

case class DualConfig(
    var vertexBits: Int = 15,
    var weightBits: Int = 30,
    var graph: SingleGraph = null
) {
  def vertexNum = graph.vertex_num.toInt
  def edgeNum = graph.weighted_edges.length.toInt
  def instructionBits = 2 * vertexBits + 2

  def this(filename: String) = {
    this()
    // read graph from file
    val source = scala.io.Source.fromFile(filename)
    val json_content =
      try source.getLines.mkString
      finally source.close()
    graph = decode[SingleGraph](json_content) match {
      case Right(graph) => graph
      case Left(ex)     => throw ex
    }
    fitGraph(graph)
  }

  // fit the bits to a specific decoding graph and construct connections
  def fitGraph(graph: SingleGraph): Unit = {
    this.graph = graph
    // compute the minimum bits of vertices and nodes; note that there could be
    // as many as 2x nodes than the number of vertices, so it's necessary to have enough bits
    assert(vertexNum > 0)
    val max_node_num = vertexNum * 2
    vertexBits = Util.bitsToHold(max_node_num)
    val max_weight = graph.weighted_edges.map(e => e.w).max
    assert(max_weight > 0)
    weightBits = Util.bitsToHold(max_weight.toInt)
    assert(weightBits <= 30)
    if (vertexBits * 2 < weightBits) {
      vertexBits = (weightBits + 1) / 2 // expand vertexBits so that the instruction can hold the maximum length
    }
    // build vertex to neighbor edge mapping
    updateIncidentEdges()
  }

  private val incidentEdges = collection.mutable.Map[Int, Seq[Int]]()
  def updateIncidentEdges(): Unit = {
    incidentEdges.clear
    for ((edge, edgeIndex) <- graph.weighted_edges.zipWithIndex) {
      for (vertexIndex <- Seq(edge.l.toInt, edge.r.toInt)) {
        if (incidentEdges.contains(vertexIndex)) {
          incidentEdges(vertexIndex) = incidentEdges(vertexIndex) :+ edgeIndex
        } else {
          incidentEdges(vertexIndex) = Seq(edgeIndex)
        }
      }
    }
  }

  def numIncidentEdgeOf(vertexIndex: Int): Int = {
    return incidentEdgesOf(vertexIndex).length
  }
  def incidentEdgesOf(vertexIndex: Int): Seq[Int] = {
    return incidentEdges(vertexIndex)
  }
  def localIndexOfEdge(vertexIndex: Int, edgeIndex: Int): Int = {
    for ((localEdgeIndex, localIndex) <- incidentEdges(vertexIndex).zipWithIndex) {
      if (localEdgeIndex == edgeIndex) {
        return localIndex
      }
    }
    throw new Exception("cannot find edge in the incident list of vertex")
  }
  def localIndexOfVertex(edgeIndex: Int, vertexIndex: Int): Int = {
    val weightedEdge = graph.weighted_edges(edgeIndex)
    if (weightedEdge.l == vertexIndex) {
      return 0
    }
    if (weightedEdge.r == vertexIndex) {
      return 1
    }
    throw new Exception("the edge does not connect the vertex")
  }

  def sanityCheck(): Unit = {
    assert(vertexBits <= 15)
    assert(vertexBits > 0)
    assert(weightBits <= 30)
    assert(weightBits > 0)
    assert(weightBits + 2 <= instructionBits)
  }
}

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
    val internal_instruction = out(InternalInstruction(config))
  }

  io.state := DualAcceleratorState.Normal
  io.internal_instruction.connect(io.instruction)

  // instantiate vertices and edges
  val vertices = Seq
    .range(0, config.vertexNum)
    .map(vertexIndex => new Vertex(config, vertexIndex))

  val edges = Seq
    .range(0, config.edgeNum)
    .map(edgeIndex => new Edge(config, edgeIndex))

  // connect the vertices and edges
  for (vertexIndex <- Range(0, config.vertexNum)) {
    val vertex = vertices(vertexIndex)
    for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
      val edge = edges(edgeIndex)
      val localIndexOfVertex = config.localIndexOfVertex(edgeIndex, vertexIndex)
      val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
      vertex.io.vertexOutputs(localIndexOfEdge) <> edge.io.vertexInputs(localIndexOfVertex)
      vertex.io.edgeInputs(localIndexOfEdge) <> edge.io.edgeOutputs(localIndexOfVertex)
    }
  }

  // TODO: gather the results in a tree structure. tip: use reduceBalancedTree function
  // https://spinalhdl.github.io/SpinalDoc-RTD/master/SpinalHDL/Data%20types/Vec.html#lib-helper-functions
}
