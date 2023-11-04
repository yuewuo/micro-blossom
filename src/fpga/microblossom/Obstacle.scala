package microblossom

import spinal.core._
import spinal.lib._
import util._

case class Obstacle(config: DualConfig = DualConfig()) extends Bits {
  val spec = config.obstacleSpec
  setWidth(spec.numBits)

  def connectFields(obstacle: Obstacle) = {
    field1 := obstacle.field1.resized
    field2 := obstacle.field2.resized
    field3 := obstacle.field3.resized
    field4 := obstacle.field4.resized
    field5 := obstacle.field5.resized
    field6 := obstacle.field6.resized
    if (spec.numBits > obstacle.spec.numBits) {
      when(obstacle.field1 === obstacle.field1.asUInt.maxValue) {
        field1(field1.getWidth - 1 downto obstacle.field1.getWidth).setAll()
      }
      when(obstacle.field2 === obstacle.field2.asUInt.maxValue) {
        field2(field2.getWidth - 1 downto obstacle.field2.getWidth).setAll()
      }
      when(obstacle.field3 === obstacle.field3.asUInt.maxValue) {
        field3(field3.getWidth - 1 downto obstacle.field3.getWidth).setAll()
      }
      when(obstacle.field4 === obstacle.field4.asUInt.maxValue) {
        field4(field4.getWidth - 1 downto obstacle.field4.getWidth).setAll()
      }
    }
  }

  def widthConvertedFrom(obstacle: Obstacle) = {
    rspCode := obstacle.rspCode
    switch(obstacle.rspCode.asUInt) {
      is(RspCode.NonZeroGrow) {
        sliceOf(spec.lengthRange) := obstacle.length.asBits.resized
        if (spec.numBits > obstacle.spec.numBits) {
          // extending length should also extend the MSB
          when(obstacle.length === obstacle.length.maxValue) {
            sliceOf(spec.lengthRange)(length.getWidth - 1 downto obstacle.length.getWidth).setAll()
          }
        }
        if (config.weightBits < 6 * config.vertexBits) { lengthZero.clearAll() }
      }
      is(RspCode.BlossomNeedExpand) {
        connectFields(obstacle)
      }
      is(RspCode.Conflict) {
        connectFields(obstacle)
      }
      is(RspCode.Reserved) {
        connectFields(obstacle)
      }
    }
  }

  def rspCode = sliceOf(spec.rspCodeRange)
  def length = sliceOf(spec.lengthRange).asUInt
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

  def dynNonZeroGrow(length: UInt): Bits = {
    B(0, numBits bits) | rspCodeRange.dynMasked(B(RspCode.NonZeroGrow, 2 bits)).resize(numBits) |
      lengthRange.dynMasked(length.asBits).resize(numBits)
  }
  def dynConflict(node1: Bits, node2: Bits, touch1: Bits, touch2: Bits, vertex1: Bits, vertex2: Bits): Bits = {
    B(0, numBits bits) | rspCodeRange.dynMasked(B(RspCode.Conflict, 2 bits)).resize(numBits) |
      field1Range.dynMasked(node1).resize(numBits) | field2Range.dynMasked(node2).resize(numBits) |
      field3Range.dynMasked(touch1).resize(numBits) | field4Range.dynMasked(touch2).resize(numBits) |
      field5Range.dynMasked(vertex1).resize(numBits) | field6Range.dynMasked(vertex2).resize(numBits)
  }

}

case class ObstacleReader(config: DualConfig, obstacle: BigInt) {
  val spec = config.obstacleSpec

  def sliceOf(range: BitRange): BigInt = {
    val mask = (BigInt(1) << (range.msb - range.lsb + 1)) - 1
    mask & (obstacle >> range.lsb)
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
}
