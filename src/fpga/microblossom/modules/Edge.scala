package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import microblossom.combinatorial._
import microblossom.util.Vivado
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

case class Edge(config: DualConfig, edgeIndex: Int) extends Component {
  val (leftVertex, rightVertex) = config.incidentVerticesOf(edgeIndex)
  val leftGrownBits = config.grownBitsOf(leftVertex)
  val rightGrownBits = config.grownBitsOf(rightVertex)

  val io = new Bundle {
    val message = in(BroadcastMessage(config))
    // interaction I/O
    val stageOutputs = out(Edge.getStages(config).getStageOutput)
    val leftVertexInput = in(Vertex.getStages(config, leftVertex).getStageOutput)
    val rightVertexInput = in(Vertex.getStages(config, rightVertex).getStageOutput)
    // final outputs
    val maxGrowable = out(ConvergecastMaxGrowable(config.weightBits))
    val conflict = out(ConvergecastConflict(config.vertexBits))
  }

  val stages = Edge.getStages(config)
  stages.connectStageOutput(io.stageOutputs)

  // fetch
  var ram: Mem[EdgeState] = null
  var register = (!config.hardCodeWeights) generate Reg(EdgeState(config.weightBits, config.hardCodeWeights))
  if (!config.hardCodeWeights) {
    register.init(EdgeState.resetValue(config, edgeIndex))
  }
  var fetchState = EdgeState(config.weightBits)
  var message = BroadcastMessage(config)
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    if (config.hardCodeWeights) {
      fetchState := EdgeState.resetValue(config, edgeIndex)
    } else {
      ram = Mem(EdgeState(config.weightBits), config.contextDepth)
      ram.setTechnology(ramBlock)
      fetchState := ram.readSync(
        address = io.message.contextId,
        enable = io.message.valid
      )
    }
    message := RegNext(io.message)
  } else {
    if (config.hardCodeWeights) {
      fetchState := EdgeState.resetValue(config, edgeIndex)
    } else {
      fetchState := register
    }
    message := io.message
  }

  stages.offloadSet.state := fetchState
  stages.offloadSet.compact.connect(message)

  stages.offloadSet2.connect(stages.offloadGet) // weight might be changed later
  val hasLayerFusion = config.edgeConditionedVertex.contains(edgeIndex)
  val conditionedVertexIsVirtual = if (hasLayerFusion) {
    val conditionedVertex = config.edgeConditionedVertex(edgeIndex)
    if (conditionedVertex == leftVertex) {
      io.leftVertexInput.offloadGet.state.isVirtual
    } else if (conditionedVertex == rightVertex) {
      io.rightVertexInput.offloadGet.state.isVirtual
    } else {
      throw new Exception("cannot find the conditioned vertex")
    }
  } else {
    False
  }
  // weight might be changed because of layer fusion
  val offload2Weight = UInt(config.weightBits bits)
  offload2Weight := stages.offloadGet.state.weight
  if (hasLayerFusion) {
    when(conditionedVertexIsVirtual) {
      offload2Weight := (stages.offloadGet.state.weight |>> 2) |<< 1
      stages.offloadSet2.state.weight := offload2Weight
    }
  }
  val offload2Area = new Area {
    val edgeIsTight = EdgeIsTight(
      leftGrownBits = leftGrownBits,
      rightGrownBits = rightGrownBits,
      weightBits = config.weightBits
    )
    edgeIsTight.io.leftGrown := io.leftVertexInput.offloadGet.state.grown
    edgeIsTight.io.rightGrown := io.rightVertexInput.offloadGet.state.grown
    edgeIsTight.io.weight := offload2Weight
    stages.offloadSet2.isTight := edgeIsTight.io.isTight
    if (hasLayerFusion) {
      stages.offloadSet2.isTightExFusion := edgeIsTight.io.isTight && !conditionedVertexIsVirtual
    } else {
      stages.offloadSet2.isTightExFusion := edgeIsTight.io.isTight
    }
  }

  stages.offloadSet3.connect(stages.offloadGet2)

  stages.offloadSet4.connect(stages.offloadGet3)

  // TODO: dynamically update edge weights
  // Mux(message.isReset, EdgeState.resetValue(config, edgeIndex), fetchState)
  stages.executeSet.connect(stages.offloadGet4)

  stages.executeSet2.connect(stages.executeGet)

  stages.executeSet3.connect(stages.executeGet2)
  val execute3Area = new Area {
    val edgeIsTight = EdgeIsTight(
      leftGrownBits = leftGrownBits,
      rightGrownBits = rightGrownBits,
      weightBits = config.weightBits
    )
    edgeIsTight.io.leftGrown := io.leftVertexInput.executeGet2.state.grown
    edgeIsTight.io.rightGrown := io.rightVertexInput.executeGet2.state.grown
    edgeIsTight.io.weight := stages.executeGet2.state.weight
    stages.executeSet3.isTight := edgeIsTight.io.isTight
  }

  stages.updateSet.connect(stages.executeGet3)
  val updateArea = new Area {
    val edgeRemaining = EdgeRemaining(
      leftGrownBits = leftGrownBits,
      rightGrownBits = rightGrownBits,
      weightBits = config.weightBits
    )
    edgeRemaining.io.leftGrown := io.leftVertexInput.executeGet3.state.grown
    edgeRemaining.io.rightGrown := io.rightVertexInput.executeGet3.state.grown
    edgeRemaining.io.weight := stages.executeGet3.state.weight
    stages.updateSet.remaining := edgeRemaining.io.remaining
  }

  stages.updateSet2.connect(stages.updateGet)

  stages.updateSet3.connect(stages.updateGet2)

  val edgeResponse = EdgeResponse(config.vertexBits, config.weightBits)
  edgeResponse.io.leftShadow := io.leftVertexInput.updateGet3.shadow
  edgeResponse.io.rightShadow := io.rightVertexInput.updateGet3.shadow
  edgeResponse.io.leftIsVirtual := io.leftVertexInput.updateGet3.state.isVirtual
  edgeResponse.io.rightIsVirtual := io.rightVertexInput.updateGet3.state.isVirtual
  edgeResponse.io.leftVertex := leftVertex
  edgeResponse.io.rightVertex := rightVertex
  edgeResponse.io.remaining := stages.updateGet3.remaining

  // 1 cycle delay when context is used (to ensure read latency >= execute latency)
  val outDelay = (config.contextDepth != 1).toInt
  io.maxGrowable := Delay(edgeResponse.io.maxGrowable, outDelay)
  io.conflict := Delay(edgeResponse.io.conflict, outDelay)

  /* No write back: giving ports for external edge weights channels */
  // val writeState = stages.updateGet3.state
  // if (config.contextBits > 0) {
  //   ram.write(
  //     address = stages.updateGet3.compact.contextId,
  //     data = writeState,
  //     enable = stages.updateGet3.compact.valid
  //   )
  // } else {
  //   when(stages.updateGet3.compact.valid) {
  //     if (!config.hardCodeWeights) {
  //       register := writeState
  //     }
  //   }
  // }

  // inject registers
  for (stageName <- config.injectRegisters) {
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

// sbt "runMain microblossom.modules.EdgeEstimation"
object EdgeEstimation extends App {
  def dualConfig(name: String): DualConfig = {
    DualConfig(filename = s"./resources/graphs/example_$name.json"),
  }
  val configurations = List(
    // 7xLUT6, 1xLUT5, 3xLUT4 -> 11
    (dualConfig("code_capacity_d5"), 1, "code capacity 2 neighbors"),
    // 9xLUT6, 3xLUT4 -> 12
    (dualConfig("code_capacity_rotated_d5"), 12, "code capacity 4 neighbors"),
    // 6xLUT6, 4xLUT5, 5xLUT4, 1xLUT3 -> 16
    (dualConfig("phenomenological_rotated_d5"), 141, "phenomenological 6 neighbors"),
    // 19xLUT6, 9xLUT4, 5xLUT2, 1xCARRY4 -> 34
    (dualConfig("circuit_level_d5"), 365, "circuit-level 12 neighbors"),
    // 18xLUT6, 2xLUT5, 11xLUT4, 4xLUT2, 1xCARRY4 -> 36
    (dualConfig("circuit_level_d11"), 4719, "circuit-level 12 neighbors")
  )
  for ((config, edgeIndex, name) <- configurations) {
    val reports = Vivado.report(Edge(config, edgeIndex))
    println(s"$name:")
    reports.resource.primitivesTable.print()
  }
}
