package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class ConvergecastMaxLength(weightBits: Int) extends Bundle {
  val length = UInt(weightBits bits)
}

case class ConvergecastConflict(vertexBits: Int) extends Bundle {
  val valid = Bool
  val field1 = Bits(vertexBits bits)
  val field2 = Bits(vertexBits bits)
  val field3 = Bits(vertexBits bits)
  val field4 = Bits(vertexBits bits)
  val field5 = Bits(vertexBits bits)
  val field6 = Bits(vertexBits bits)
}
