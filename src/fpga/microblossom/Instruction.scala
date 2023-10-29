package microblossom

import spinal.core._
import spinal.lib._
import util._

// defines the I/O interface: it's always 32 bit width
object InstructionIO extends Instruction(DualConfig()) {
  assert(InstructionIO.config.instructionBits == 32)

  /* helper functions for simulation purpose */
  def setSpeed(node: UInt, speed: Speed): Bits = {
    B(32 bits, (31 downto 17) -> node.resized)
    // opCode #= OpCode.SetSpeed.value.toInt
    // field1 #= node.toInt
  }
}

case class Instruction(config: DualConfig = DualConfig()) extends Bits {
  setWidth(config.instructionBits)

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

  val opCodeRange = BitRange(1, 0)
  def opCode = sliceOf(opCodeRange)
  val isExtendedRange = BitRange(2, 2)
  def isExtended = sliceOf(isExtendedRange)
  val extendedOpCodeRange = BitRange(5, 3)
  def extendedOpCode = sliceOf(extendedOpCodeRange)
  val lengthRange = BitRange(config.instructionBits - 1, 2)
  def length = sliceOf(lengthRange)
  val field1Range = BitRange(config.instructionBits - 1, config.instructionBits - config.vertexBits)
  def field1 = sliceOf(field1Range)
  val field2Range = BitRange(config.instructionBits - config.vertexBits - 1, 2)
  def field2 = sliceOf(field2Range)
  val extendedField2Range = BitRange(config.instructionBits - config.vertexBits - 1, 6)
  def extendedField2 = sliceOf(extendedField2Range)
  val speedRange =
    BitRange(config.instructionBits - config.vertexBits - 1, config.instructionBits - config.vertexBits - 2)
  def speed = sliceOf(speedRange)
  def setSpeedZeroRange = BitRange(config.instructionBits - config.vertexBits - 3, 2)
  def setSpeedZero = sliceOf(setSpeedZeroRange)

  def sliceOf(range: BitRange): Bits = {
    this(range.msb downto range.lsb)
  }

}

case class BitRange(msb: Int, lsb: Int) {
  assert(msb >= lsb)
}
