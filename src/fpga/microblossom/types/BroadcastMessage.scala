package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class BroadcastMessage(config: DualConfig, explicitReset: Boolean = true) extends Bundle {
  val valid = Bool
  val instruction = Instruction(config)
  val isReset = explicitReset generate Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def resizedFrom(source: BroadcastMessage) = {
    valid := source.valid
    instruction.resizedFrom(source.instruction)
    if (explicitReset) {
      isReset := source.isReset
    }
    if (config.contextBits > 0) {
      contextId := source.contextId
    }
  }
}

case class BroadcastCompact(config: DualConfig) extends Bundle {
  val valid = Bool
  val isReset = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  def connect(message: BroadcastMessage) = {
    valid := message.valid
    isReset := message.isReset
    if (config.contextBits > 0) {
      contextId := message.contextId
    }
  }
}
