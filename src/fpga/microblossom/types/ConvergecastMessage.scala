package microblossom.types

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
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

  def assignReordered(source: ConvergecastConflict) = {
    val IndexNone = (1 << vertexBits) - 1
    valid := source.valid
    when(source.node1 === IndexNone) {
      node1 := source.node2
      node2 := source.node1
      touch1 := source.touch2
      touch2 := source.touch1
      vertex1 := source.vertex2
      vertex2 := source.vertex1
    } otherwise {
      node1 := source.node1
      node2 := source.node2
      touch1 := source.touch1
      touch2 := source.touch2
      vertex1 := source.vertex1
      vertex2 := source.vertex2
    }
  }
}

case class DataMaxGrowable(
    var length: Int
)

@ConfiguredJsonCodec
case class DataConflict(
    var valid: Boolean,
    var node1: Int,
    var node2: Option[Int],
    var touch1: Int,
    var touch2: Option[Int],
    var vertex1: Int,
    var vertex2: Int
)

object DataConflict {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

@ConfiguredJsonCodec
case class DataConflictRaw(
    var valid: Boolean,
    var node1: Int,
    var node2: Int,
    var touch1: Int,
    var touch2: Int,
    var vertex1: Int,
    var vertex2: Int
)

object DataConflictRaw {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

@ConfiguredJsonCodec
case class DataPreMatching(
    var edgeIndex: Int,
    var node1: Int,
    var node2: Option[Int],
    var touch1: Int,
    var touch2: Option[Int],
    var vertex1: Int,
    var vertex2: Int
)

object DataPreMatching {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
