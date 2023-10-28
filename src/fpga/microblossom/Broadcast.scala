package microblossom

import spinal.core._
import spinal.lib._
import util._

case class Instruction(config: DualConfig) extends Bundle {
  val raw = Bits(config.instructionBits bits)

  def connect(instruction: Bits): Unit = {
    assert(instruction.getWidth == 32) // the instruction must be 32 bits
    val opcode_1 = instruction(1 downto 0)
    raw(1 downto 0) := opcode_1
    switch(opcode_1) {
      is(Architecture.OpCode1.SetSpeed) {
        raw(1 + config.vertexBits downto 2) := instruction(1 + config.vertexBits downto 2)
        raw(1 + 2 * config.vertexBits downto 2 + config.vertexBits) := instruction(
          16 + config.vertexBits downto 17
        )
      }
      default {
        raw(1 + 2 * config.vertexBits downto 2) := B"0".resized
      }
    }
  }

  def opcode = raw(1 downto 0)

  // for testing purpose
//   def generateInstruction(opCode: OpCode): InternalInstruction = {}
}
