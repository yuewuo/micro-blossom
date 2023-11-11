package microblossom.vertex

import spinal.core._
import spinal.lib._
import microblossom._

case class VertexPersistent(config: DualConfig, vertexIndex: Int) extends Bundle {
  val speed = Speed()
  val node = Bits(config.vertexBits bits)
  val root = Bits(config.vertexBits bits)
  val isVirtual = Bool
  val isDefect = Bool
  val grown = UInt(config.grownBitsOf(vertexIndex) bits)
}
