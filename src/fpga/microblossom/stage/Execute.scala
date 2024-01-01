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

  def connect(last: StageOffloadVertex4) = {
    state := last.state
    message := last.message
  }
}

case class StageExecuteVertex2(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val isStalled = Bool
  // throw away the broadcast message because it's not used later on
  val compact = BroadcastCompact(config)

  def connect(last: StageExecuteVertex) = {
    isStalled := last.isStalled
    compact.connect(last.message)
  }
}

case class StageExecuteVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val isStalled = Bool
  val compact = BroadcastCompact(config)

  def connect(last: StageExecuteVertex2) = {
    state := last.state
    isStalled := last.isStalled
    compact := last.compact
  }
}

/*
 * Edge
 */

case class StageExecuteEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val compact = BroadcastCompact(config)

  def connect(last: StageOffloadEdge4) = {
    state := last.state
    compact := last.compact
  }
}

case class StageExecuteEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val compact = BroadcastCompact(config)

  def connect(last: StageExecuteEdge) = {
    state := last.state
    compact := last.compact
  }
}

case class StageExecuteEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
  val compact = BroadcastCompact(config)

  def connect(last: StageExecuteEdge2) = {
    state := last.state
    compact := last.compact
  }
}
