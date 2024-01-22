package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class ConvergecastMaxGrowable(weightBits: Int) extends Bundle {
  val length = UInt(weightBits bits)

  def resizedFrom(source: ConvergecastMaxGrowable) = {
    require(length.getBitsWidth >= source.length.getBitsWidth)
    when(source.length === source.length.maxValue) {
      length := length.maxValue
    } otherwise {
      length := source.length.resized
    }
  }
}

case class ConvergecastConflict(vertexBits: Int) extends Bundle {
  val node1 = Bits(vertexBits bits)
  val node2 = Bits(vertexBits bits)
  val touch1 = Bits(vertexBits bits)
  val touch2 = Bits(vertexBits bits)
  val vertex1 = Bits(vertexBits bits)
  val vertex2 = Bits(vertexBits bits)
  val valid = Bool

  def resizedFrom(source: ConvergecastConflict) = {
    valid := source.valid
    def resizeConnectUp(source: Bits, target: Bits) = {
      target := source.resized
      if (target.getWidth > source.getWidth) {
        when(source === (1 << source.getWidth) - 1) {
          target(target.getWidth - 1 downto source.getWidth).setAll()
        }
      }
    }
    resizeConnectUp(source.node1, node1)
    resizeConnectUp(source.node2, node2)
    resizeConnectUp(source.touch1, touch1)
    resizeConnectUp(source.touch2, touch2)
    vertex1 := source.vertex1.resized
    vertex2 := source.vertex2.resized
  }
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
