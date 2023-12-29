package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import org.scalatest.funsuite.AnyFunSuite

object Vertex {
  def getStages(
      config: DualConfig,
      vertexIndex: Int
  ): Stages[
    StageOffloadVertex,
    StageOffloadVertex2,
    StageOffloadVertex3,
    StageOffloadVertex4,
    StageExecuteVertex,
    StageExecuteVertex2,
    StageExecuteVertex3,
    StageUpdateVertex,
    StageUpdateVertex2,
    StageUpdateVertex3
  ] = {
    Stages(
      offload = () => StageOffloadVertex(config, vertexIndex),
      offload2 = () => StageOffloadVertex2(config, vertexIndex),
      offload3 = () => StageOffloadVertex3(config, vertexIndex),
      offload4 = () => StageOffloadVertex4(config, vertexIndex),
      execute = () => StageExecuteVertex(config, vertexIndex),
      execute2 = () => StageExecuteVertex2(config, vertexIndex),
      execute3 = () => StageExecuteVertex3(config, vertexIndex),
      update = () => StageUpdateVertex(config, vertexIndex),
      update2 = () => StageUpdateVertex2(config, vertexIndex),
      update3 = () => StageUpdateVertex3(config, vertexIndex)
    )
  }
}

case class Vertex(config: DualConfig, vertexIndex: Int, injectRegisters: Seq[String] = List()) extends Component {
  val io = new Bundle {
    val message = in(BroadcastMessage(config))
    // interaction I/O
    val stageOutputs = out(Vertex.getStages(config, vertexIndex).getStageOutput)
    val edgeInputs = in(
      Vec(
        for (edgeIndex <- config.incidentEdgesOf(vertexIndex))
          yield Edge.getStages(config).getStageOutput
      )
    )
    val offloaderInputs = in(
      Vec(
        for (offloaderIndex <- config.incidentOffloaderOf(vertexIndex))
          yield Offloader.getStages(config, offloaderIndex).getStageOutput
      )
    )
    // final outputs
  }

  val stages = Vertex.getStages(config, vertexIndex)
  stages.connectStageOutput(io.stageOutputs)

  // fetch
  var ram: Mem[VertexState] = null
  var register = Reg(VertexState(config.vertexBits, config.grownBitsOf(vertexIndex)))
  register.speed init (Speed.Stay)
  register.node init (config.IndexNone)
  register.root init (config.IndexNone)
  register.isVirtual init (config.isVirtual(vertexIndex))
  register.isDefect init (false)
  register.grown init (0)
  var fetchState = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  var message = BroadcastMessage(config)
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(VertexState(config.vertexBits, config.grownBitsOf(vertexIndex)), config.contextDepth)
    fetchState := ram.readSync(
      address = io.message.contextId,
      enable = io.message.valid
    )
    message := RegNext(io.message)
  } else {
    fetchState := register
    message := io.message
  }

  // mock
  stages.offloadSet.message := message
  stages.offloadSet.state := fetchState

  stages.offloadSet2.connect(stages.offloadGet)

  stages.offloadSet3.connect(stages.offloadGet2)
  stages.offloadSet3.isUniqueTight := False // TODO

  stages.offloadSet4.connect(stages.offloadGet3)

  stages.executeSet.connect(stages.offloadGet4)
  stages.executeSet.isStalled := False // TODO

  stages.executeSet2.connect(stages.executeGet)
  stages.executeSet2.state := stages.executeGet.state // TODO

  stages.executeSet3.connect(stages.executeGet2)

  stages.updateSet.connect(stages.executeGet3)
  stages.updateSet.propagatingPeer.valid := False // TODO
  stages.updateSet.propagatingPeer.node := config.IndexNone // TODO
  stages.updateSet.propagatingPeer.root := config.IndexNone // TODO

  stages.updateSet2.connect(stages.updateGet)
  stages.updateSet2.state := stages.updateGet.state // TODO
  stages.updateSet2.shadow.speed := Speed.Stay // TODO
  stages.updateSet2.shadow.node := config.IndexNone // TODO
  stages.updateSet2.shadow.root := config.IndexNone // TODO

  stages.updateSet3.connect(stages.updateGet2)

  // write back
  if (config.contextBits > 0) {
    ram.write(
      address = stages.updateGet3.contextId,
      data = stages.updateGet3.state,
      enable = stages.updateGet3.valid
    )
  } else {
    when(stages.updateGet3.valid) {
      register := stages.updateGet3.state
    }
  }

  // inject registers
  for (stageName <- injectRegisters) {
    stages.injectRegisterAt(stageName)
  }
  stages.finish()

}

// sbt 'testOnly microblossom.modules.VertexTest'
class VertexTest extends AnyFunSuite {

  test("construct a Vertex") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Vertex(config, 0))
  }

}
