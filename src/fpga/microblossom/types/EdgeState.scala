package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class EdgeState(weightBits: Int) extends Bundle {
  val weight = UInt(weightBits bits)
}
