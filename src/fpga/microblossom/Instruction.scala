package microblossom

import spinal.core._
import spinal.lib._
import util._

case class Instruction(config: DualConfig = DualConfig()) extends Bits {
  val spec = config.instructionSpec
  setWidth(spec.numBits)

  def widthConvertedFrom(instruction: Instruction) = {
    opCode := instruction.opCode
    switch(instruction.opCode.asUInt) {
      is(OpCode.SetBlossom) {
        field1 := instruction.field1.resized // node
        field2 := instruction.field2.resized // blossom
      }
      is(OpCode.Match) {
        field1 := instruction.field1.resized // vertex_1
        field2 := instruction.field2.resized // vertex_2
      }
      is(OpCode.SetSpeed) {
        when(instruction.extensionIndicator === B"0") {
          field1 := instruction.field1.resized
          speed := instruction.speed
          setSpeedZero.clearAll()
        } otherwise {
          field1 := instruction.field1.resized
          extendedField2 := instruction.extendedField2.resized
          extendedOpCode := instruction.extendedOpCode
          extensionIndicator := instruction.extensionIndicator
        }
      }
      is(OpCode.Grow) {
        sliceOf(spec.lengthRange) := instruction.length.asBits.resized // length
        if (config.weightBits < 2 * config.vertexBits) { growZero.clearAll() }
      }
    }
  }

  def opCode = sliceOf(spec.opCodeRange)
  def extensionIndicator = sliceOf(spec.extensionIndicatorRange)
  def extendedOpCode = sliceOf(spec.extendedOpCodeRange)
  def length = sliceOf(spec.lengthRange).asUInt
  def growZero = if (config.weightBits < 2 * config.vertexBits) sliceOf(spec.growZeroRange) else null
  def payload = sliceOf(spec.payloadRange)
  def field1 = sliceOf(spec.field1Range)
  def field2 = sliceOf(spec.field2Range)
  def extendedPayload = sliceOf(spec.extendedPayloadRange)
  def extendedField2 = sliceOf(spec.extendedField2Range)
  def speed = sliceOf(spec.speedRange)
  def setSpeedZero = sliceOf(spec.setSpeedZeroRange)

  def sliceOf(range: BitRange): Bits = {
    this(range.msb downto range.lsb)
  }

  def isSetSpeed(): Bool = (opCode === OpCode.SetSpeed) && (extensionIndicator.asBool === False)
  def isExtended(): Bool = (opCode === OpCode.SetSpeed) && (extensionIndicator.asBool === True)
  def isSetBlossom(): Bool = (opCode === OpCode.SetBlossom)
  def isGrow(): Bool = (opCode === OpCode.Grow)
  def isAddDefect(): Bool = isExtended && (extendedOpCode === ExtendedOpCode.AddDefectVertex)
  def isFindObstacle(): Bool = isExtended && (extendedOpCode === ExtendedOpCode.FindObstacle)
  def isReset(): Bool = isExtended && (extendedOpCode === ExtendedOpCode.Reset)
}

case class BitRange(msb: Int, lsb: Int) {
  assert(msb >= lsb)
  def numBits = msb - lsb + 1

  def masked(value: Long): Long = {
    assert(numBits > 0)
    assert(value >= 0)
    val maxValue = 1 << numBits
    assert(value < maxValue)
    value << lsb
  }

  def dynMasked(value: Bits): Bits = {
    assert(value.getWidth <= numBits)
    value << lsb
  }
}

case class InstructionSpec(config: DualConfig) {
  def numBits = 2 * config.vertexBits + 2

  def opCodeRange = BitRange(1, 0)
  def extensionIndicatorRange = BitRange(2, 2)
  def extendedOpCodeRange = BitRange(5, 3)
  def lengthRange = BitRange(config.weightBits + 1, 2)
  def growZeroRange = BitRange(numBits - 1, config.weightBits + 2)
  def payloadRange = BitRange(numBits - 1, 2)
  def field1Range = BitRange(numBits - 1, numBits - config.vertexBits)
  def field2Range = BitRange(numBits - config.vertexBits - 1, 2)
  def extendedPayloadRange = BitRange(numBits, 6)
  def extendedField2Range = BitRange(numBits - config.vertexBits - 1, 6)
  def speedRange =
    BitRange(numBits - config.vertexBits - 1, numBits - config.vertexBits - 2)
  def setSpeedZeroRange = BitRange(numBits - config.vertexBits - 3, 2)

  def generateSetSpeed(node: Long, speed: Long): Long = {
    opCodeRange.masked(OpCode.SetSpeed) | field1Range.masked(node) | speedRange.masked(speed)
  }
  def generateSetBlossom(node: Long, blossom: Long): Long = {
    opCodeRange.masked(OpCode.SetBlossom) | field1Range.masked(node) | speedRange.masked(blossom)
  }
  def generateExtendedSuffix(extendedOpCode: Long): Long = {
    opCodeRange.masked(OpCode.SetSpeed) | extensionIndicatorRange.masked(1) | extendedOpCodeRange.masked(extendedOpCode)
  }
  def generateReset(): Long = {
    generateExtendedSuffix(ExtendedOpCode.Reset)
  }
  def generateFindObstacle(): Long = {
    generateExtendedSuffix(ExtendedOpCode.FindObstacle)
  }
  def generateAddDefect(vertex: Long, node: Long): Long = {
    generateExtendedSuffix(ExtendedOpCode.AddDefectVertex) | field1Range.masked(vertex) |
      extendedField2Range.masked(node)
  }
  def generateGrow(length: Long): Long = {
    opCodeRange.masked(OpCode.Grow) | lengthRange.masked(length)
  }

  def sanityCheck() = {
    assert(config.weightBits + 2 <= numBits)
  }
}
