package microblossom.stage

import microblossom._
import microblossom.types._
import spinal.core._
import spinal.lib._

/*
 * Vertex
 */

case class StageExecuteVertex(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)
  val isStalled = Bool
}

case class StageExecuteVertex2(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val isStalled = Bool
}

case class StageExecuteVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val isStalled = Bool
}

/*
 * Edge
 */

case class StageExecuteEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}

case class StageExecuteEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}

case class StageExecuteEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
}
