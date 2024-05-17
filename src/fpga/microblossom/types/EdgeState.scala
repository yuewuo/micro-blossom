package microblossom.types

import spinal.core._
import spinal.lib._
import microblossom._

case class EdgeState(weightBits: Int, hardCodeWeights: Boolean = false) extends Bundle {
  val weight = (!hardCodeWeights) generate UInt(weightBits bits)
}

object EdgeState {
  def resetValue(config: DualConfig, edgeIndex: Int): EdgeState = {
    val reset = EdgeState(config.weightBits)
    reset.weight := config.graph.weighted_edges(edgeIndex).w.toInt
    reset
  }
}
