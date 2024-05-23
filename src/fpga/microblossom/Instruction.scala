package microblossom

import spinal.core._
import spinal.lib._
import util._

case class Instruction(config: DualConfig = DualConfig()) extends Bits {
  val spec = config.instructionSpec
  setWidth(spec.numBits)

  def resizedFrom(source: Instruction): Unit = {
    if (source.config == config) {
      this := source
      return
    }
    opCode := source.opCode
    switch(source.opCode.asUInt) {
      is(OpCode.SetBlossom) {
        field1 := source.field1.resized // node
        field2 := source.field2.resized // blossom
      }
      is(OpCode.Match) {
        field1 := source.field1.resized // vertex_1
        field2 := source.field2.resized // vertex_2
      }
      is(OpCode.SetSpeed) {
        when(source.extensionIndicator === B"0") {
          field1 := source.field1.resized
          speed := source.speed
          setSpeedZero.clearAll()
        } otherwise {
          extendedOpCode := source.extendedOpCode
          extensionIndicator := source.extensionIndicator
          when(source.extendedOpCode.asUInt === ExtendedOpCode.Grow) {
            extendedPayload := source.extendedPayload.resized
          } otherwise {
            field1 := source.field1.resized
            extendedField2 := source.extendedField2.resized
          }
        }
      }
      is(OpCode.AddDefectVertex) {
        field1 := source.field1.resized // vertex_1
        field2 := source.field2.resized // vertex_2
      }
    }
  }

  def opCode = sliceOf(spec.opCodeRange)
  def extensionIndicator = sliceOf(spec.extensionIndicatorRange)
  def extendedOpCode = sliceOf(spec.extendedOpCodeRange)
  def length = sliceOf(spec.lengthRange).asUInt
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
  def isGrow(): Bool = isExtended && (extendedOpCode === ExtendedOpCode.Grow)
  def isAddDefect(): Bool = opCode === OpCode.AddDefectVertex
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
  def payloadRange = BitRange(numBits - 1, 2)
  def field1Range = BitRange(numBits - 1, numBits - config.vertexBits)
  def field2Range = BitRange(numBits - config.vertexBits - 1, 2)
  def lengthRange = BitRange(config.weightBits + 5, 6)
  def extendedPayloadRange = BitRange(numBits - 1, 6)
  def extendedField2Range = BitRange(numBits - config.vertexBits - 1, 6)
  def speedRange =
    BitRange(numBits - config.vertexBits - 1, numBits - config.vertexBits - 2)
  def setSpeedZeroRange = BitRange(numBits - config.vertexBits - 3, 2)

  def generateSetSpeed(node: Long, speed: Long): Long = {
    opCodeRange.masked(OpCode.SetSpeed) | field1Range.masked(node) | speedRange.masked(speed)
  }
  def generateSetBlossom(node: Long, blossom: Long): Long = {
    opCodeRange.masked(OpCode.SetBlossom) | field1Range.masked(node) | field2Range.masked(blossom)
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
    opCodeRange.masked(OpCode.AddDefectVertex) | field1Range.masked(vertex) | field2Range.masked(node)
  }
  def generateGrow(length: Long): Long = {
    generateExtendedSuffix(ExtendedOpCode.Grow) | extendedPayloadRange.masked(length)
  }

  def dynamicGrow(length: UInt, config: DualConfig = DualConfig()): Instruction = {
    val instruction = Instruction(config)
    instruction.opCode := OpCode.SetSpeed
    instruction.extensionIndicator := True.asBits
    instruction.extendedOpCode := ExtendedOpCode.Grow
    instruction.extendedPayload.clearAll()
    instruction.length := length
    instruction
  }

  def sanityCheck() = {
    assert(config.weightBits + 2 <= numBits)
  }
}
