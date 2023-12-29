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

  def connect(last: StageOffloadVertex) = {
    state := last.state
    message := last.message
  }
}

case class StageOffloadVertex3(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)
  val isUniqueTight = Bool

  def connect(last: StageOffloadVertex2) = {
    state := last.state
    message := last.message
  }
}

case class StageOffloadVertex4(config: DualConfig, vertexIndex: Int) extends Bundle {
  val state = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  val message = BroadcastMessage(config)

  def connect(last: StageOffloadVertex3) = {
    state := last.state
    message := last.message
  }
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
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

case class StageOffloadEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageOffloadEdge) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}

case class StageOffloadEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageOffloadEdge2) = {
    state := last.state
    isTight := last.isTight
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}

case class StageOffloadEdge4(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(last: StageOffloadEdge3) = {
    state := last.state
    valid := last.valid
    if (config.contextBits > 0) {
      contextId := last.contextId
    }
  }
}
