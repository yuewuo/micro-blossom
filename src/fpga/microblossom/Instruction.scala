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
  def isLoadDefectsExternal(): Bool = isExtended && (extendedOpCode === ExtendedOpCode.LoadDefectsExternal)

  def assignGrow(length: UInt) = {
    opCode := OpCode.SetSpeed
    extensionIndicator := True.asBits
    extendedOpCode := ExtendedOpCode.Grow
    if (spec.lengthRange.msb < spec.numBits - 1) {
      sliceOf(BitRange(spec.numBits - 1, spec.lengthRange.msb + 1)).assignDontCare()
    }
    val lengthBits = sliceOf(spec.lengthRange) // must use Bits assign...
    lengthBits := length.asBits.resized
  }
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

  def of(value: Long): Long = {
    assert(numBits > 0)
    assert(value >= 0)
    val mask = (1 << numBits) - 1
    (value >> lsb) & mask
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
  def generateLoadDefectsExternal(time: Long): Long = {
    generateExtendedSuffix(ExtendedOpCode.LoadDefectsExternal) | field1Range.masked(time)
  }

  def sanityCheck() = {
    assert(config.weightBits + 2 <= numBits)
  }

  // fields of instruction
  def opCode(value: Long) = opCodeRange.of(value)
  def extensionIndicator(value: Long) = (extensionIndicatorRange.of(value) != 0)
  def extendedOpCode(value: Long) = extendedOpCodeRange.of(value)
  def length(value: Long) = lengthRange.of(value)
  def payload(value: Long) = payloadRange.of(value)
  def field1(value: Long) = field1Range.of(value)
  def field2(value: Long) = field2Range.of(value)
  def extendedPayload(value: Long) = extendedPayloadRange.of(value)
  def extendedField2(value: Long) = extendedField2Range.of(value)
  def speed(value: Long) = speedRange.of(value)
  def setSpeedZero(value: Long) = setSpeedZeroRange.of(value)

  def isSetSpeed(value: Long) = (opCode(value) == OpCode.SetSpeed) && !extensionIndicator(value)
  def isExtended(value: Long) = (opCode(value) == OpCode.SetSpeed) && extensionIndicator(value)
  def isSetBlossom(value: Long) = (opCode(value) == OpCode.SetBlossom)
  def isGrow(value: Long) = isExtended(value) && (opCode(value) == ExtendedOpCode.Grow)
  def isAddDefect(value: Long) = opCode(value) == OpCode.AddDefectVertex
  def isFindObstacle(value: Long) = isExtended(value) && (extendedOpCode(value) == ExtendedOpCode.FindObstacle)
  def isReset(value: Long) = isExtended(value) && (extendedOpCode(value) == ExtendedOpCode.Reset)
  def isLoadDefectsExternal(value: Long) =
    isExtended(value) && (extendedOpCode(value) == ExtendedOpCode.LoadDefectsExternal)

  def isValid(value: Long): Boolean = {
    value < (1L << numBits)
  }

  def binaryOf(value: Long): String = {
    String.format("%" + numBits + "s", value.toBinaryString).replace(' ', '0')
  }

  def format(value: Long): String = {
    assert(isValid(value))
    if (isSetSpeed(value)) {
      return s"SetSpeed(node=${field1(value)}, speed=${Speed.format(speed(value))})"
    } else if (isSetBlossom(value)) {
      return s"SetBlossom(node=${field1(value)}, blossom=${field2(value)})"
    } else if (isGrow(value)) {
      return s"Grow(length=${length(value)})"
    } else if (isAddDefect(value)) {
      return s"AddDefectVertex(vertex=${field1(value)}, node=${field2(value)})"
    } else if (isFindObstacle(value)) {
      return s"FindObstacle()"
    } else if (isReset(value)) {
      return s"Reset()"
    } else if (isLoadDefectsExternal(value)) {
      return s"LoadDefectsExternal(time=${field1(value)})"
    } else {
      return s"Unknown(value=${value}=0b${binaryOf(value)})"
    }
  }

  // convert from value in another spec; default is the 32 bit instruction
  def from(value: Long, spec: InstructionSpec = InstructionSpec(DualConfig())): Long = {
    spec.toSpec(value, this)
  }

  def toSpec(value: Long, spec: InstructionSpec): Long = {
    assert(isValid(value))
    if (isSetSpeed(value)) {
      val result = spec.generateSetSpeed(field1(value), speed(value))
      assert(spec.field1(result) == field1(value))
      assert(spec.speed(result) == speed(value))
      return result
    } else if (isSetBlossom(value)) {
      val result = spec.generateSetBlossom(field1(value), field2(value))
      assert(spec.field1(result) == field1(value))
      assert(spec.field2(result) == field2(value))
      return result
    } else if (isGrow(value)) {
      val result = spec.generateGrow(length(value))
      assert(spec.length(result) == length(value))
      return result
    } else if (isAddDefect(value)) {
      val result = spec.generateAddDefect(field1(value), field2(value))
      assert(spec.field1(result) == field1(value))
      assert(spec.field2(result) == field2(value))
      return result
    } else if (isFindObstacle(value)) {
      return spec.generateFindObstacle()
    } else if (isReset(value)) {
      return spec.generateReset()
    } else if (isLoadDefectsExternal(value)) {
      val result = spec.generateLoadDefectsExternal(field1(value))
      assert(spec.field1(result) == field1(value))
      return result
    } else {
      throw new Exception(s"Unknown(value=${value}=0b${binaryOf(value)})")
    }
  }
}
