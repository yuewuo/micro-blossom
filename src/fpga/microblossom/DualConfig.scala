package microblossom

import spinal.core._
import util._
import io.circe.parser.decode
import collection.mutable.ArrayBuffer

case class DualConfig(
    var vertexBits: Int = 15,
    var weightBits: Int = 30,
    var grownBits: Int = 30,
    var broadcastDelay: Int = 1,
    var convergecastDelay: Int = 1,
    var contextDepth: Int = 1, // how many different contexts are supported
    // optional features
    var supportAddDefectVertex: Boolean = true,
    // load graph either from parameter or from file
    var graph: SingleGraph = null,
    val filename: String = null,
    val minimizeBits: Boolean = true
) {
  def vertexNum = graph.vertex_num.toInt
  def edgeNum = graph.weighted_edges.length.toInt
  def offloaderNum = graph.offloading.length.toInt
  def instructionSpec = InstructionSpec(this)
  def obstacleSpec = ObstacleSpec(this)
  def contextBits = log2Up(contextDepth)
  def IndexNone = (1 << vertexBits) - 1
  def LengthNone = (1 << weightBits) - 1
  def readLatency = broadcastDelay + convergecastDelay + 5 // from sending the command to receiving the obstacle
  private val virtualVertices = collection.mutable.Set[Int]()
  private val incidentEdges = collection.mutable.Map[Int, ArrayBuffer[Int]]() // vertexIndex -> Seq[edgeIndex]
  private val incidentOffloaders = collection.mutable.Map[Int, ArrayBuffer[Int]]() // vertexIndex -> Seq[offloaderIdnex]

  if (filename != null) {
    val source = scala.io.Source.fromFile(filename)
    val json_content =
      try source.getLines.mkString
      finally source.close()
    assert(graph == null, "cannot provide both graph and filename")
    graph = decode[SingleGraph](json_content) match {
      case Right(graph) => graph
      case Left(ex)     => throw ex
    }
    fitGraph(minimizeBits)
  } else if (graph != null) {
    fitGraph(minimizeBits)
  }

  // fit the bits to a specific decoding graph and construct connections
  def fitGraph(minimizeBits: Boolean = true): Unit = {
    // compute the minimum bits of vertices and nodes; note that there could be
    // as many as 2x nodes than the number of vertices, so it's necessary to have enough bits
    assert(vertexNum > 0)
    if (minimizeBits) {
      val max_node_num = vertexNum * 2
      vertexBits = log2Up(max_node_num)
      val max_weight = graph.weighted_edges.map(e => e.w).max
      assert(max_weight > 0)
      // weightBits = log2Up(max_weight.toInt * graph.weighted_edges.length)
      weightBits = log2Up(max_weight.toInt + 1) // weightBits could be smaller than grownBits
      assert(weightBits <= 30)
      if (vertexBits * 2 < weightBits) {
        vertexBits = (weightBits + 1) / 2 // expand vertexBits so that the instruction can hold the maximum length
      }
      if (vertexBits < 5) {
        vertexBits = 5 // at least 5 bits to support all instructions
      }
    }
    // build vertex to neighbor edge mapping
    updateIncidentEdges()
    updateIncidentOffloaders()
    // update virtual vertices
    virtualVertices.clear()
    for (vertexIndex <- graph.virtual_vertices) {
      virtualVertices += vertexIndex.toInt
    }
  }

  def updateIncidentEdges() = {
    incidentEdges.clear()
    for ((edge, edgeIndex) <- graph.weighted_edges.zipWithIndex) {
      for (vertexIndex <- Seq(edge.l.toInt, edge.r.toInt)) {
        if (!incidentEdges.contains(vertexIndex)) {
          incidentEdges(vertexIndex) = ArrayBuffer()
        }
        incidentEdges(vertexIndex).append(edgeIndex)
      }
    }
  }
  def updateIncidentOffloaders() = {
    incidentOffloaders.clear()
    for ((offloader, offloaderIndex) <- graph.offloading.zipWithIndex) {
      for (vertexIndex <- offloaderNeighborVertexIndices(offloaderIndex)) {
        if (!incidentOffloaders.contains(vertexIndex)) {
          incidentOffloaders(vertexIndex) = ArrayBuffer()
        }
        incidentOffloaders(vertexIndex).append(offloaderIndex)
      }
    }
  }

  def numIncidentEdgeOf(vertexIndex: Int): Int = {
    return incidentEdgesOf(vertexIndex).length
  }
  def incidentEdgesOf(vertexIndex: Int): Seq[Int] = {
    return incidentEdges(vertexIndex)
  }
  def incidentOffloaderOf(vertexIndex: Int): Seq[Int] = {
    return incidentOffloaders(vertexIndex)
  }
  def numIncidentOffloaderOf(vertexIndex: Int): Int = {
    return incidentOffloaderOf(vertexIndex).length
  }
  def incidentVerticesOf(edgeIndex: Int): (Int, Int) = {
    val edge = graph.weighted_edges(edgeIndex)
    return (edge.l.toInt, edge.r.toInt)
  }
  def incidentVerticesPairsOf(edgeIndex: Int): Seq[Seq[Int]] = {
    val edge = graph.weighted_edges(edgeIndex)
    return Seq(
      Seq(edge.l.toInt, edge.r.toInt),
      Seq(edge.r.toInt, edge.l.toInt)
    )
  }
  def peerVertexOfEdge(edgeIndex: Int, vertexIndex: Int): Int = {
    val edge = graph.weighted_edges(edgeIndex)
    if (edge.l.toInt == vertexIndex) {
      return edge.r.toInt
    } else if (edge.r.toInt == vertexIndex) {
      return edge.l.toInt
    } else {
      throw new Exception(s"vertex $vertexIndex is not incident to edge $edgeIndex")
    }
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
    val edge = graph.weighted_edges(edgeIndex)
    if (edge.l == vertexIndex) {
      return 0
    }
    if (edge.r == vertexIndex) {
      return 1
    }
    throw new Exception("the edge does not connect the vertex")
  }
  def isVirtual(vertexIndex: Int): Boolean = {
    virtualVertices.contains(vertexIndex)
  }
  def grownBitsOf(vertexIndex: Int): Int = {
    log2Up(graph.vertex_max_growth(vertexIndex) + 1).max(weightBits)
  }
  // (edgeIndex, neighborVertices, neighborEdges)
  def offloaderInformation(offloaderIndex: Int): (Int, Seq[Int], Seq[Int]) = {
    val offloader = graph.offloading(offloaderIndex)
    offloader.dm match {
      case Some(defectMatch) =>
        val edgeIndex = defectMatch.e.toInt
        val (left, right) = incidentVerticesOf(edgeIndex)
        return (edgeIndex, Seq(left, right), Seq())
      case None =>
    }
    offloader.vm match {
      case Some(virtualMatch) =>
        val edgeIndex = virtualMatch.e.toInt
        val edge = graph.weighted_edges(edgeIndex)
        val virtualVertex = virtualMatch.v.toInt
        val regularVertex = peerVertexOfEdge(edgeIndex, virtualVertex)
        val neighborEdges = incidentEdgesOf(regularVertex)
        return (
          edgeIndex,
          neighborEdges.map(ei => peerVertexOfEdge(ei, regularVertex)).filter(_ != virtualVertex) ++
            List(virtualVertex, regularVertex),
          neighborEdges.filter(_ != edgeIndex)
        )
      case None =>
    }
    throw new Exception("unrecognized definition of offloader")
  }
  def offloaderEdgeIndex(offloaderIndex: Int): Int = {
    val (edgeIndex, neighborVertices, neighborEdges) = offloaderInformation(offloaderIndex)
    edgeIndex
  }
  def offloaderNeighborVertexIndices(offloaderIndex: Int): Seq[Int] = {
    val (edgeIndex, neighborVertices, neighborEdges) = offloaderInformation(offloaderIndex)
    neighborVertices
  }
  def offloaderNeighborEdgeIndices(offloaderIndex: Int): Seq[Int] = {
    val (edgeIndex, neighborVertices, neighborEdges) = offloaderInformation(offloaderIndex)
    neighborEdges
  }
  def numOffloaderNeighborOf(offloaderIndex: Int): Int = {
    offloaderNeighborVertexIndices(offloaderIndex).length
  }

  def sanityCheck(): Unit = {
    assert(vertexBits <= 15)
    assert(vertexBits > 0)
    assert(weightBits <= 30)
    assert(weightBits > 0)
    assert(contextDepth > 0)
    instructionSpec.sanityCheck()
  }
}
