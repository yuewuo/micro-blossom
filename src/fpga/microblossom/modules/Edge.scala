package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import org.scalatest.funsuite.AnyFunSuite

object Edge {
  def getStages(
      config: DualConfig
  ): Stages[
    StageOffloadEdge,
    StageOffloadEdge2,
    StageOffloadEdge3,
    StageOffloadEdge4,
    StageExecuteEdge,
    StageExecuteEdge2,
    StageExecuteEdge3,
    StageUpdateEdge,
    StageUpdateEdge2,
    StageUpdateEdge3
  ] = {
    Stages(
      offload = () => StageOffloadEdge(config),
      offload2 = () => StageOffloadEdge2(config),
      offload3 = () => StageOffloadEdge3(config),
      offload4 = () => StageOffloadEdge4(config),
      execute = () => StageExecuteEdge(config),
      execute2 = () => StageExecuteEdge2(config),
      execute3 = () => StageExecuteEdge3(config),
      update = () => StageUpdateEdge(config),
      update2 = () => StageUpdateEdge2(config),
      update3 = () => StageUpdateEdge3(config)
    )
  }
}

case class Edge(config: DualConfig, edgeIndex: Int, injectRegisters: Seq[String] = List()) extends Component {
  val (leftVertex, rightVertex) = config.incidentVerticesOf(edgeIndex)
  val io = new Bundle {
    val message = in(BroadcastMessage(config))
    val stageOutputs = out(Edge.getStages(config).getStageOutput)
    val leftVertexInput = in(Vertex.getStages(config, leftVertex).getStageOutput)
    val rightVertexInput = in(Vertex.getStages(config, rightVertex).getStageOutput)
  }

  val stages = Edge.getStages(config)
  stages.connectStageOutput(io.stageOutputs)

  // fetch
  var ram: Mem[EdgeState] = null
  var register = Reg(EdgeState(config.weightBits))
  var fetchState = EdgeState(config.weightBits)
  // var message = BroadcastMessage(config)
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(EdgeState(config.weightBits), config.contextDepth)
    fetchState := ram.readSync(
      address = io.message.contextId,
      enable = io.message.valid
    )
    // message := RegNext(io.message)
  } else {
    fetchState := register
    // message := io.message
  }

  // mock
  stages.offloadSet.state := fetchState

  stages.offloadSet2.state := stages.offloadGet.state // TODO
  stages.offloadSet2.isTight := False // TODO

  stages.offloadSet3.state := stages.offloadGet2.state // TODO
  stages.offloadSet3.isTight := stages.offloadGet2.isTight // TODO

  stages.offloadSet4.state := stages.offloadGet3.state // TODO

  stages.executeSet.state := stages.offloadGet4.state // TODO

  stages.executeSet2.state := stages.executeGet.state // TODO

  stages.executeSet3.state := stages.executeGet2.state // TODO
  stages.executeSet3.isTight := False // TODO

  stages.updateSet.state := stages.executeGet3.state // TODO

  stages.updateSet2.state := stages.updateGet.state // TODO

  stages.updateSet3.state := stages.updateGet2.state // TODO
  stages.updateSet3.remaining := 0 // TODO

  // TODO: write back
  register := stages.updateGet3.state

  // inject registers
  for (stageName <- injectRegisters) {
    stages.injectRegisterAt(stageName)
  }
  stages.finish()

}

// sbt 'testOnly microblossom.modules.EdgeTest'
class EdgeTest extends AnyFunSuite {

  test("construct a Edge") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Edge(config, 0))
  }

}
