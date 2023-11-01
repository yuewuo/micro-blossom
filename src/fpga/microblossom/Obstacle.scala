package microblossom

import spinal.core._
import spinal.lib._
import util._

case class Obstacle(config: DualConfig = DualConfig()) extends Bits {
  val spec = config.obstacleSpec
  setWidth(spec.numBits)

  def widthConvertedFrom(obstacle: Obstacle) = {
    rspCode := obstacle.rspCode
    switch(obstacle.rspCode.asUInt) {
      is(RspCode.NonZeroGrow) {
        length := obstacle.length.resized
        if (config.weightBits < 6 * config.vertexBits) { lengthZero.clearAll() }
      }
      is(RspCode.Conflict) {
        field1 := obstacle.field1.resized
        field2 := obstacle.field2.resized
        field3 := obstacle.field3.resized
        field4 := obstacle.field4.resized
        field5 := obstacle.field5.resized
        field6 := obstacle.field6.resized
      }
      is(RspCode.BlossomNeedExpand) {
        field1 := obstacle.field1.resized
        field2 := obstacle.field2.resized
        field3 := obstacle.field3.resized
        field4 := obstacle.field4.resized
        field5 := obstacle.field5.resized
        field6 := obstacle.field6.resized
      }
      is(RspCode.Reserved) {
        field1 := obstacle.field1.resized
        field2 := obstacle.field2.resized
        field3 := obstacle.field3.resized
        field4 := obstacle.field4.resized
        field5 := obstacle.field5.resized
        field6 := obstacle.field6.resized
      }
    }
  }

  def rspCode = sliceOf(spec.rspCodeRange)
  def length = sliceOf(spec.lengthRange)
  def lengthZero = if (config.weightBits < 6 * config.vertexBits) sliceOf(spec.lengthZeroRange) else null
  def payload = sliceOf(spec.payloadRange)
  def field1 = sliceOf(spec.field1Range)
  def field2 = sliceOf(spec.field2Range)
  def field3 = sliceOf(spec.field3Range)
  def field4 = sliceOf(spec.field4Range)
  def field5 = sliceOf(spec.field5Range)
  def field6 = sliceOf(spec.field6Range)

  def sliceOf(range: BitRange): Bits = {
    this(range.msb downto range.lsb)
  }

  def isNonZeroGrow(): Bool = (rspCode === RspCode.NonZeroGrow)
  def isConflict(): Bool = (rspCode === RspCode.Conflict)
  def isBlossomNeedExpand(): Bool = (rspCode === RspCode.BlossomNeedExpand)
}

case class ObstacleSpec(config: DualConfig) {
  def numBits = 6 * config.vertexBits + 2

  def rspCodeRange = BitRange(1, 0)
  def lengthRange = BitRange(config.weightBits + 1, 2)
  def lengthZeroRange = BitRange(numBits - 1, config.weightBits + 2)
  def payloadRange = BitRange(numBits - 1, 2)
  def field1Range = BitRange(numBits - 1, numBits - config.vertexBits)
  def field2Range = BitRange(numBits - config.vertexBits - 1, numBits - config.vertexBits * 2)
  def field3Range = BitRange(numBits - config.vertexBits * 2 - 1, numBits - config.vertexBits * 3)
  def field4Range = BitRange(numBits - config.vertexBits * 3 - 1, numBits - config.vertexBits * 4)
  def field5Range = BitRange(numBits - config.vertexBits * 4 - 1, numBits - config.vertexBits * 5)
  def field6Range = BitRange(numBits - config.vertexBits * 5 - 1, numBits - config.vertexBits * 6)

  def generateNonZeroGrow(length: Long): Long = {
    rspCodeRange.masked(RspCode.NonZeroGrow) | lengthRange.masked(length)
  }
  def generateConflict(node1: Long, node2: Long, touch1: Long, touch2: Long, vertex1: Long, vertex2: Long): Long = {
    rspCodeRange.masked(RspCode.Conflict) | field1Range.masked(node1) | field2Range.masked(node2) |
      field3Range.masked(touch1) | field4Range.masked(touch2) | field5Range.masked(vertex1) |
      field6Range.masked(vertex2)
  }
  def generateBlossomNeedExpand(blossom: Long): Long = {
    rspCodeRange.masked(RspCode.BlossomNeedExpand) | field1Range.masked(blossom)
  }
}
