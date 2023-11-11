package microblossom.vertex

import spinal.core._
import spinal.lib._
import microblossom._

case class VertexFetchStageInput(config: DualConfig, vertexIndex: Int) extends Bundle {
  val message = BroadcastMessage(config)
}

case class VertexFetchStageOutput(config: DualConfig, vertexIndex: Int) extends Bundle {
  val message = BroadcastMessage(config)
}

case class VertexFetchStage(config: DualConfig, vertexIndex: Int) extends Component {
  val io = new Bundle {
    val input = VertexFetchStageInput(config, vertexIndex)
    val output = VertexFetchStageOutput(config, vertexIndex)
  }

}
