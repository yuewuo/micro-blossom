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

  def connect(instruction: Instruction): Unit = {
    assert(instruction.getWidth == 32) // the instruction must be 32 bits
    sliceOf(opCode1) := instruction.sliceOf(instruction.opCode1)
    switch(sliceOf(opCode1)) {
      is(Architecture.OpCode1.SetSpeed) {
        this(1 + config.vertexBits downto 2) := instruction(1 + config.vertexBits downto 2)
        this(1 + 2 * config.vertexBits downto 2 + config.vertexBits) := instruction(
          16 + config.vertexBits downto 17
        )
      }
    }
  }

  def opCode1 = Field(1, 0)
  def sliceOf(field: Field): Bits = {
    this(field.msb downto field.lsb)
  }

  // def opCode = this(1 downto 0)
  // def field1 = this(config.instructionBits - 1 downto config.instructionBits - config.vertexBits)
  // def field2 = this(config.instructionBits - config.vertexBits - 1 downto 2)
  // def speed = this(config.instructionBits - config.vertexBits - 1 downto config.instructionBits - config.vertexBits - 2)
  // def length = this(config.instructionBits - 1 downto 2)

  def delayed = {
    val instruction = Instruction(config)
    instruction.assignFromBits(RegNext(asBits))
    instruction
  }

  // def

}

case class Field(msb: Int, lsb: Int) {}
