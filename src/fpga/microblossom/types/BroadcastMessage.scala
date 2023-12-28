package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class BroadcastMessage(config: DualConfig) extends Bundle {
  val valid = Bool
  val instruction = Instruction(config)
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}
