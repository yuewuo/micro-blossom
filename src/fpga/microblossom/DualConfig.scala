package microblossom

import spinal.core._
import util._
import io.circe.parser.decode
import collection.mutable.ArrayBuffer
import org.scalatest.funsuite.AnyFunSuite

object DualConfig {
  def version = Integer.parseInt("24" + "01" + "23" + "c0", 16) // year - month - date - 'c'revision
}

case class DualConfig(
    var vertexBits: Int = 15,
    var weightBits: Int = 26,
    var broadcastDelay: Int = 0,
    var convergecastDelay: Int = 1, // the write or register update takes 1 clock cycle, so delay the output by 1
    var instructionBufferDepth: Int = 4, // buffer write instructions for higher throughput, must be a power of 2
    var contextDepth: Int = 1, // how many different contexts are supported
    var conflictChannels: Int = 1, // how many conflicts are collected at once in parallel
    var hardCodeWeights: Boolean = true, // hard-code the edge weights to simplify logic
    // optional features
    var supportAddDefectVertex: Boolean = true,
    var supportOffloading: Boolean = false,
    var supportLayerFusion: Boolean = false,
    var supportLoadStallEmulator: Boolean = false,
    // load graph either from parameter or from file
    var graph: SingleGraph = null,
    val filename: String = null,
    val minimizeBits: Boolean = true,
    var injectRegisters: Seq[String] = List()
) {
  assert(isPow2(instructionBufferDepth) & instructionBufferDepth >= 2)
  if (supportLoadStallEmulator) {
    assert(supportLayerFusion)
  }

  def vertexNum = graph.vertex_num.toInt
  def edgeNum = graph.weighted_edges.length.toInt
  def offloaderNum = activeOffloading.length.toInt
  def instructionSpec = InstructionSpec(this)
  def contextBits = log2Up(contextDepth)
  def instructionBufferBits = log2Up(2 * instructionBufferDepth + 2) // the dual module may be processing an instruction
  def IndexNone = (1 << vertexBits) - 1
  def LengthNone = (1 << weightBits) - 1
  def supportContextSwitching = contextBits > 0
  def executeLatency = { // from sending the command to the time it's safe to write to the same context again
    // when context switching, 2 cycles delay due to memory fetch and write
    val contextDelay = 2 * (contextDepth != 1).toInt
    injectRegisters.length + contextDelay
  }
  def readLatency = { // from sending the command to receiving the obstacle
    broadcastDelay + convergecastDelay + executeLatency
  }
  def layerFusion = {
    graph.layer_fusion match {
      case Some(layer_fusion) => layer_fusion
      case None               => LayerFusion(0, Seq(), Map(), Map(), Map())
    }
  }
  def parityReporters = {
    graph.parity_reporters match {
      case Some(parity_reporters) => parity_reporters.reporters
      case None                   => Seq()
    }
  }
  def parityReportersNum = parityReporters.length.toInt

  private val virtualVertices = collection.mutable.Set[Int]()
  private val incidentEdges = collection.mutable.Map[Int, ArrayBuffer[Int]]() // vertexIndex -> Seq[edgeIndex]
  private val incidentOffloaders = collection.mutable.Map[Int, ArrayBuffer[Int]]() // vertexIndex -> Seq[offloaderIndex]
  val activeOffloading = ArrayBuffer[Offloading]()
  val edgeConditionedVertex = collection.mutable.Map[Int, Int]()
  val vertexLayerId = collection.mutable.Map[Int, Int]()
  var layerIdBits: Int = 0

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
      assert(weightBits <= 26)
      if (weightBits > vertexBits * 2 - 4) {
        vertexBits = (weightBits + 5) / 2 // expand vertexBits so that the instruction can hold the maximum length
      }
      if (vertexBits < 5) {
        vertexBits = 5 // at least 5 bits to support all instructions
      }
    }
    // update virtual vertices
    virtualVertices.clear()
    for (vertexIndex <- graph.virtual_vertices) {
      virtualVertices += vertexIndex.toInt
    }
    // build vertex to neighbor edge mapping
    updateIncidentEdges()
    updateOffloading()
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
  def updateOffloading(): Unit = {
    incidentOffloaders.clear()
    activeOffloading.clear()
    edgeConditionedVertex.clear()
    vertexLayerId.clear()
    var maxLayerId = 0
    if (supportOffloading) {
      for (offloading <- graph.offloading) {
        activeOffloading.append(offloading)
      }
    }
    if (supportLayerFusion) {
      for ((edgeIndex, conditionedVertex) <- layerFusion.fusion_edges) {
        if (supportOffloading) {
          activeOffloading.append(Offloading(fm = Some(FusionMatch(edgeIndex, conditionedVertex))))
        }
        edgeConditionedVertex(edgeIndex.toInt) = conditionedVertex.toInt
      }
      for ((vertexIndex, layerId) <- layerFusion.vertex_layer_id) {
        vertexLayerId(vertexIndex.toInt) = layerId.toInt
        maxLayerId = maxLayerId.max(layerId.toInt)
      }
      layerIdBits = log2Up(maxLayerId)
    }
    for ((offloader, offloaderIndex) <- activeOffloading.zipWithIndex) {
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
    return incidentEdges.getOrElse(vertexIndex, Seq())
  }
  def incidentOffloaderOf(vertexIndex: Int): Seq[Int] = {
    return incidentOffloaders.getOrElse(vertexIndex, Seq())
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
    for ((localEdgeIndex, localIndex) <- incidentEdgesOf(vertexIndex).zipWithIndex) {
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
  def offloaderTypeOf(offloaderIndex: Int): String = {
    val offloader = activeOffloading(offloaderIndex)
    offloader.dm match {
      case Some(defectMatch) =>
        return "defect_match"
      case None =>
    }
    offloader.vm match {
      case Some(virtualMatch) =>
        return "virtual_match"
      case None =>
    }
    throw new Exception("unrecognized definition of offloader")
  }
  // (edgeIndex, neighborVertices, neighborEdges)
  def offloaderInformation(offloaderIndex: Int): (Int, Seq[Int], Seq[Int]) = {
    val offloader = activeOffloading(offloaderIndex)
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
    offloader.fm match {
      case Some(fusionMatch) =>
        val edgeIndex = fusionMatch.e.toInt
        val conditionalVertex = fusionMatch.c.toInt
        val regularVertex = peerVertexOfEdge(edgeIndex, conditionalVertex)
        return (edgeIndex, Seq(conditionalVertex, regularVertex), Seq())
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

// sbt 'testOnly *DualConfigTest'
class DualConfigTest extends AnyFunSuite {

  test("construct config manually") {
    val config = DualConfig(vertexBits = 4, weightBits = 3)
    config.sanityCheck()
  }

  test("construct config incorrectly") {
    assertThrows[AssertionError] {
      // if the weight consists of too many bits to fit into a single message
      val config = new DualConfig(vertexBits = 4, weightBits = 10)
      config.sanityCheck()
    }
  }

  test("construct config from file") {
    val config = new DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    config.sanityCheck()
    assert(config.localIndexOfEdge(vertexIndex = 0, edgeIndex = 0) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 1, edgeIndex = 0) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 1, edgeIndex = 1) == 1)
    assert(config.localIndexOfEdge(vertexIndex = 2, edgeIndex = 1) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 3, edgeIndex = 2) == 0)
    assert(config.localIndexOfEdge(vertexIndex = 0, edgeIndex = 2) == 1)
    assertThrows[Exception] { // exception when the edge is not incident to the vertex
      assert(config.localIndexOfEdge(vertexIndex = 0, edgeIndex = 1) == 0)
    }
  }

}
