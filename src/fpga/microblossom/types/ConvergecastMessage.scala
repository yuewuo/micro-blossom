package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class ConvergecastMaxGrowable(weightBits: Int) extends Bundle {
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

case class DataMaxGrowable(
    var length: Int
)

case class DataConflict(
    var valid: Boolean,
    var node1: Int,
    var node2: Int,
    var touch1: Int,
    var touch2: Int,
    var vertex1: Int,
    var vertex2: Int
)
