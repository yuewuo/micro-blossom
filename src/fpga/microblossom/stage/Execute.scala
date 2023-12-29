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

case class StageExecute2Edge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}

case class StageExecute3Edge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
}
