package microblossom.stage

import microblossom._
import microblossom.types._
import spinal.core._
import spinal.lib._

/*
 * Vertex
 */

case class StageOffloadVertex(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)
}

case class StageOffloadVertex2(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)
}

case class StageOffloadVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)
  val isUniqueTight = Bool
}

case class StageOffloadVertex4(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)
}

/*
 * Offloader
 */

case class StageOffloadOffloader4(numNeighbors: Int) extends Bundle {
  val stallVertex = Vec.fill(numNeighbors)(Bool)
}

/*
 * Edge
 */

case class StageOffloadEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}

case class StageOffloadEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
}

case class StageOffloadEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
}

case class StageOffloadEdge4(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
}
