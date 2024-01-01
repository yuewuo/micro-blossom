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

case class StageOffloadOffloader4(config: DualConfig, offloaderIndex: Int) extends Bundle {
  val stallVertex = Vec.fill(config.numOffloaderNeighborOf(offloaderIndex))(Bool)

  def getVertexIsStalled(targetVertexIndex: Int): Bool = {
    for ((vertexIndex, localIndex) <- config.offloaderNeighborVertexIndices(offloaderIndex).zipWithIndex) {
      if (vertexIndex == targetVertexIndex) {
        return stallVertex(localIndex)
      }
    }
    throw new Exception(s"offloader $offloaderIndex cannot find vertex $targetVertexIndex")
  }
}

/*
 * Edge
 */

case class StageOffloadEdge(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val compact = BroadcastCompact(config)
}

case class StageOffloadEdge2(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
  val compact = BroadcastCompact(config)

  def connect(last: StageOffloadEdge) = {
    state := last.state
    compact := last.compact
  }
}

case class StageOffloadEdge3(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val isTight = Bool
  val compact = BroadcastCompact(config)

  def connect(last: StageOffloadEdge2) = {
    state := last.state
    isTight := last.isTight
    compact := last.compact
  }
}

case class StageOffloadEdge4(config: DualConfig) extends Bundle {
  val state = EdgeState(config.weightBits)
  val compact = BroadcastCompact(config)

  def connect(last: StageOffloadEdge3) = {
    state := last.state
    compact := last.compact
  }
}
