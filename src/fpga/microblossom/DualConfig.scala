package microblossom

import spinal.core._
import util._
import io.circe.parser.decode

case class DualConfig(
    var vertexBits: Int = 15,
    var weightBits: Int = 30,
    var graph: SingleGraph = null,
    var broadcastDelay: Int = 1,
    var convergecastDelay: Int = 1
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
    if (vertexBits < 4) {
      vertexBits = 4 // at least 4 bits to support all instructions
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
