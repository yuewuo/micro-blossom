package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class ConvergecastMaxLength(weightBits: Int) extends Bundle {
  val length = UInt(weightBits bits)
}

case class ConvergecastConflict(vertexBits: Int) extends Bundle {
  val valid = Bool
  val node1 = Bits(vertexBits bits)
  val node2 = Bits(vertexBits bits)
  val touch1 = Bits(vertexBits bits)
  val touch2 = Bits(vertexBits bits)
  val vertex1 = Bits(vertexBits bits)
  val vertex2 = Bits(vertexBits bits)
}
