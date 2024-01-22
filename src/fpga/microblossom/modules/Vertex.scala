package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import microblossom.combinatorial._
import microblossom.util.Vivado
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

case class Vertex(config: DualConfig, vertexIndex: Int) extends Component {
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
    var peerVertexInputsExecute3 = in(
      Vec(
        for (edgeIndex <- config.incidentEdgesOf(vertexIndex))
          yield Vertex.getStages(config, config.peerVertexOfEdge(edgeIndex, vertexIndex)).executeGet3
      )
    )
    // final outputs
    val maxGrowable = out(ConvergecastMaxGrowable(config.weightBits))
  }

  val stages = Vertex.getStages(config, vertexIndex)
  stages.connectStageOutput(io.stageOutputs)

  // fetch
  var ram: Mem[VertexState] = null
  var register = Reg(VertexState(config.vertexBits, config.grownBitsOf(vertexIndex)))
  register init (VertexState.resetValue(config, vertexIndex))
  var fetchState = VertexState(config.vertexBits, config.grownBitsOf(vertexIndex))
  var message = BroadcastMessage(config)
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(VertexState(config.vertexBits, config.grownBitsOf(vertexIndex)), config.contextDepth)
    ram.setTechnology(ramBlock)
    fetchState := ram.readSync(
      address = io.message.contextId,
      enable = io.message.valid
    )
    message := RegNext(io.message)
  } else {
    fetchState := register
    message := io.message
  }

  stages.offloadSet.message := message
  stages.offloadSet.state := Mux(message.isReset, VertexState.resetValue(config, vertexIndex), fetchState)

  stages.offloadSet2.connect(stages.offloadGet)

  stages.offloadSet3.connect(stages.offloadGet2)
  var offload3Area = new Area {
    var isUniqueTight = VertexIsUniqueTight(
      numEdges = config.numIncidentEdgeOf(vertexIndex)
    )
    for (localIndex <- 0 until config.numIncidentEdgeOf(vertexIndex)) {
      isUniqueTight.io.tights(localIndex) := io.edgeInputs(localIndex).offloadGet2.isTight
    }
    stages.offloadSet3.isUniqueTight := isUniqueTight.io.isUnique
  }

  stages.offloadSet4.connect(stages.offloadGet3)

  stages.executeSet.connect(stages.offloadGet4)
  var executeArea = new Area {
    var offloadStalled = OffloadStalled(
      numConditions = config.numIncidentOffloaderOf(vertexIndex)
    )
    for (localIndex <- 0 until config.numIncidentOffloaderOf(vertexIndex)) {
      offloadStalled.io.conditions(localIndex) :=
        io.offloaderInputs(localIndex).offloadGet4.getVertexIsStalled(vertexIndex)
    }
    stages.executeSet.isStalled := offloadStalled.io.isStalled
  }

  stages.executeSet2.connect(stages.executeGet)
  var execute2Area = new Area {
    var vertexPostExecuteState = VertexPostExecuteState(
      config = config,
      vertexIndex = vertexIndex
    )
    vertexPostExecuteState.io.before := stages.executeGet.state
    vertexPostExecuteState.io.message := stages.executeGet.message
    stages.executeSet2.state := vertexPostExecuteState.io.after
  }

  stages.executeSet3.connect(stages.executeGet2)

  stages.updateSet.connect(stages.executeGet3)
  var updateArea = new Area {
    var vertexPropagatingPeer = VertexPropagatingPeer(
      config = config,
      vertexIndex = vertexIndex
    )
    vertexPropagatingPeer.io.grown := stages.executeGet3.state.grown
    for (localIndex <- 0 until config.numIncidentEdgeOf(vertexIndex)) {
      vertexPropagatingPeer.io.edgeIsTight(localIndex) := io.edgeInputs(localIndex).executeGet3.isTight
      vertexPropagatingPeer.io.peerSpeed(localIndex) := io.peerVertexInputsExecute3(localIndex).state.speed
      vertexPropagatingPeer.io.peerNode(localIndex) := io.peerVertexInputsExecute3(localIndex).state.node
      vertexPropagatingPeer.io.peerRoot(localIndex) := io.peerVertexInputsExecute3(localIndex).state.root
    }
    stages.updateSet.propagatingPeer := vertexPropagatingPeer.io.peer
  }

  stages.updateSet2.connect(stages.updateGet)
  var update2Area = new Area {
    var vertexPostUpdateState = VertexPostUpdateState(
      config = config,
      vertexIndex = vertexIndex
    )
    vertexPostUpdateState.io.before := stages.updateGet.state
    vertexPostUpdateState.io.propagator := stages.updateGet.propagatingPeer
    stages.updateSet2.state := vertexPostUpdateState.io.after

    var vertexShadow = VertexShadow(
      config = config,
      vertexIndex = vertexIndex
    )
    vertexShadow.io.node := stages.updateGet.state.node
    vertexShadow.io.root := stages.updateGet.state.root
    vertexShadow.io.speed := stages.updateGet.state.speed
    vertexShadow.io.grown := stages.updateGet.state.grown
    vertexShadow.io.isStalled := stages.updateGet.isStalled
    vertexShadow.io.propagator := stages.updateGet.propagatingPeer
    stages.updateSet2.shadow := vertexShadow.io.shadow
  }

  stages.updateSet3.connect(stages.updateGet2)

  val vertexResponse = VertexResponse(config, vertexIndex)
  vertexResponse.io.state := stages.updateGet3.state
  io.maxGrowable := vertexResponse.io.maxGrowable

  // write back
  val writeState = stages.updateGet3.state
  if (config.contextBits > 0) {
    ram.write(
      address = stages.updateGet3.compact.contextId,
      data = writeState,
      enable = stages.updateGet3.compact.valid
    )
  } else {
    when(stages.updateGet3.compact.valid) {
      register := writeState
    }
  }

  // inject registers
  for (stageName <- config.injectRegisters) {
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

// sbt "runMain microblossom.modules.VertexEstimation"
object VertexEstimation extends App {
  def dualConfig(name: String): DualConfig = {
    DualConfig(filename = s"./resources/graphs/example_$name.json"),
  }
  val configurations = List(
    // 33xLUT6, 21xLUT5, 7xLUT4, 6xLUT3, 7xLUT2 -> 74
    (dualConfig("code_capacity_d5"), 1, "code capacity 2 neighbors"),
    // 40xLUT6, 24xLUT5, 22xLUT4, 5xLUT3, 6xLUT2 -> 97
    (dualConfig("code_capacity_rotated_d5"), 10, "code capacity 4 neighbors"),
    // 37xLUT6, 73xLUT5, 21xLUT4, 10xLUT3, 8xLUT2 -> 149
    (dualConfig("phenomenological_rotated_d5"), 64, "phenomenological 6 neighbors"),
    // 42xLUT6, 107xLUT5, 31xLUT4, 18xLUT3, 8xLUT2, 2xCARRY4 -> 208
    (dualConfig("circuit_level_d5"), 63, "circuit-level 12 neighbors"),
    // 79xLUT6, 212xLUT5, 7xLUT4, 14xLUT3, 6xLUT2, 2xLUT1, 4xCARRY4 -> 324
    (dualConfig("circuit_level_d11"), 845, "circuit-level 12 neighbors (d=11)")
  )
  for ((config, vertexIndex, name) <- configurations) {
    val reports = Vivado.report(Vertex(config, vertexIndex))
    println(s"$name:")
    reports.resource.primitivesTable.print()
  }
}
