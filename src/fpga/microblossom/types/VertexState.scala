package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class VertexState(vertexBits: Int, grownBits: Int) extends Bundle {
  val speed = Speed()
  val node = Bits(vertexBits bits)
  val root = Bits(vertexBits bits)
  val isVirtual = Bool
  val isDefect = Bool
  val grown = UInt(grownBits bits)
}

object VertexState {
  def resetValue(config: DualConfig, vertexIndex: Int): VertexState = {
    val reset = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
    reset.speed := Speed.Stay
    reset.node := config.IndexNone
    reset.root := config.IndexNone
    reset.isVirtual := Bool(config.isVirtual(vertexIndex))
    reset.isDefect := False
    reset.grown := 0
    reset
  }
}
