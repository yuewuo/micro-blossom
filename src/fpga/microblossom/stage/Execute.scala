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
  // throw away the broadcast message because it's not used later on
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

case class StageExecuteVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val isStalled = Bool
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

/*
 * Edge
 */

case class StageExecuteEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageOffloadEdge4) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}

case class StageExecuteEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageExecuteEdge) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}

case class StageExecuteEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageExecuteEdge2) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}
