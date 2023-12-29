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
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

case class StageUpdateVertex2(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val shadow = VertexShadowResult(config.vertexBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

case class StageUpdateVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val shadow = VertexShadowResult(config.vertexBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

/*
 * Edge
 */

case class StageUpdateEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageExecuteEdge3) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}

case class StageUpdateEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageUpdateEdge) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}

case class StageUpdateEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val remaining = UInt(config.weightBits bits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageUpdateEdge2) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}
