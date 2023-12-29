package microblossom.stage

import microblossom._
import microblossom.types._
import microblossom.combinatorial._
import spinal.core._
import spinal.lib._

/*
 * Vertex
 */

case class StageUpdateVertex(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val isStalled = Bool
}

case class StageUpdateVertex2(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val shadow = VertexShadowResult(config.vertexBits)
}

case class StageUpdateVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val shadow = VertexShadowResult(config.vertexBits)
}

/*
 * Edge
 */

case class StageUpdateEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}

case class StageUpdateEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}

case class StageUpdateEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val remaining = UInt(config.weightBits bits)
}
