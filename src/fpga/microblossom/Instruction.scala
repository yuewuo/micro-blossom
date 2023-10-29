package microblossom

import spinal.core._
import spinal.lib._
import util._

// defines the I/O interface: it's always 32 bit width
object InstructionIO extends Instruction(DualConfig()) {
  assert(InstructionIO.config.instructionSpec.numBits == 32)

  /* helper functions for simulation purpose */
  def setSpeed(node: UInt, speed: Speed): Bits = {
    B(32 bits, (31 downto 17) -> node.resized)
    // opCode #= OpCode.SetSpeed.value.toInt
    // field1 #= node.toInt
  }
}

case class Instruction(config: DualConfig = DualConfig()) extends Bits {
  setWidth(config.instructionSpec.numBits)

  def widthConvertedFrom(instruction: Instruction): Unit = {
    opCode := instruction.opCode
    when(instruction.opCode === OpCode.SetBlossom) {
      field1 := instruction.field1.resized
      field2 := instruction.field2.resized

    } elsewhen (instruction.opCode === OpCode.Match) {
      field1 := instruction.field1.resized
      field2 := instruction.field2.resized
    } elsewhen (instruction.opCode === OpCode.SetSpeed) {
      field1 := instruction.field1.resized
      when(instruction.isExtended === B"0") {
        speed := instruction.speed
        setSpeedZero.clearAll()
      } otherwise {
        extendedField2 := instruction.extendedField2.resized
        extendedOpCode := instruction.extendedOpCode
        isExtended := instruction.isExtended
      }
    } otherwise {
      length := instruction.length.resized
    }
  }

  val spec = InstructionSpec(config)
  def opCode = sliceOf(spec.opCodeRange)
  def isExtended = sliceOf(spec.isExtendedRange)
  def extendedOpCode = sliceOf(spec.extendedOpCodeRange)
  def length = sliceOf(spec.lengthRange)
  def field1 = sliceOf(spec.field1Range)
  def field2 = sliceOf(spec.field2Range)
  def extendedField2 = sliceOf(spec.extendedField2Range)
  def speed = sliceOf(spec.speedRange)
  def setSpeedZero = sliceOf(spec.setSpeedZeroRange)

  def sliceOf(range: BitRange): Bits = {
    this(range.msb downto range.lsb)
  }

}

case class BitRange(msb: Int, lsb: Int) {
  assert(msb >= lsb)
  def numBits = msb - lsb + 1
}

case class InstructionSpec(config: DualConfig) {
  def numBits = 2 * config.vertexBits + 2

  def opCodeRange = BitRange(1, 0)
  def isExtendedRange = BitRange(2, 2)
  def extendedOpCodeRange = BitRange(5, 3)
  def lengthRange = BitRange(numBits - 1, 2)
  def field1Range = BitRange(numBits - 1, numBits - config.vertexBits)
  def field2Range = BitRange(numBits - config.vertexBits - 1, 2)
  def extendedField2Range = BitRange(numBits - config.vertexBits - 1, 6)
  def speedRange =
    BitRange(numBits - config.vertexBits - 1, numBits - config.vertexBits - 2)
  def setSpeedZeroRange = BitRange(numBits - config.vertexBits - 3, 2)

  def generateMaskedValueFor(range: BitRange, value: Long): Long = {
    assert(range.numBits > 0)
    assert(value >= 0)
    val maxValue = 1 << range.numBits
    assert(value < maxValue)
    value << range.lsb
  }
  def generateSetSpeed(node: Long, speed: Long): Long = {
    generateMaskedValueFor(opCodeRange, OpCode.SetSpeed) |
      generateMaskedValueFor(field1Range, node) | generateMaskedValueFor(speedRange, speed)
  }

  def sanityCheck() = {
    assert(config.weightBits + 2 <= numBits)
  }
}
