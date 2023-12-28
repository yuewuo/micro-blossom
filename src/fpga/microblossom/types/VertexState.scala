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
