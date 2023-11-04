package microblossom

import spinal.core._
import spinal.lib._
import microblossom._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite

// persistent state of an edge
case class EdgePersistent(config: DualConfig) extends Bundle {
  val weight = UInt(config.weightBits bits)
}

object EdgePersistent {
  def resetValue(config: DualConfig, edgeIndex: Int): EdgePersistent = {
    val reset = EdgePersistent(config)
    reset.weight := config.graph.weighted_edges(edgeIndex).w
    reset
  }
}

case class EdgeFeed(config: DualConfig) extends Bundle {
  // execute stage

  // update stage
  val updateIsTight = Bool()
  val updatePeerNode = Bits(config.vertexBits bits)
  val updatePeerRoot = Bits(config.vertexBits bits)
  val updatePeerSpeed = Speed()
  // write stage

}

case class Edge(config: DualConfig, edgeIndex: Int) extends Component {
  val io = new Bundle {
    val input = in(BroadcastMessage(config))
    val output = out(ConvergecastMessage(config))
    val edgeFeeds = out(Vec.fill(2)(EdgeFeed(config)))
    val vertexIns = in(Vec.fill(2)(VertexFeed(config)))
  }

  private var pipelineIndex = 0
  val left = io.vertexIns(0)
  val right = io.vertexIns(1)

  /*
   * pipeline input signals
   */

  val executeValid = Bool
  val executeState = EdgePersistent(config)
  val executeIsFindObstacle = Bool
  val executeIsReset = Bool
  val executeGrown = UInt(config.weightBits bits)
  val executeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val updateValid = Bool
  val updateState = EdgePersistent(config)
  val updateIsFindObstacle = Bool
  val updateIsReset = Bool
  val updateGrown = UInt(config.weightBits bits)
  val updateContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val writeValid = Bool
  val writeState = EdgePersistent(config)
  val writeIsFindObstacle = Bool
  val writeIsReset = Bool
  val writeGrown = UInt(config.weightBits bits)
  val writeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val reportIsFindObstacle = Bool
  val reportContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val reportLeftShadow = VertexShadow(config)
  val reportRightShadow = VertexShadow(config)

  // fetch stage
  var ram: Mem[EdgePersistent] = null
  var register = Reg(EdgePersistent(config))
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(EdgePersistent(config), config.contextDepth)
    executeState := ram.readSync(
      address = io.input.contextId,
      enable = io.input.valid
    )
    executeContextId := RegNext(io.input.contextId)
  } else {
    executeState := RegNext(register)
  }
  executeValid := RegNext(io.input.valid)
  executeIsFindObstacle := RegNext(io.input.valid && io.input.instruction.isFindObstacle) init False
  executeIsReset := RegNext(io.input.valid && io.input.instruction.isReset) init False
  pipelineIndex += 1

  // execute stage
  executeGrown := left.executeGrown + right.executeGrown

  val updateIsTight = RegNext(
    Mux(executeValid, executeGrown >= executeState.weight, False)
  )
  for (pair <- config.incidentVerticesPairsOf(edgeIndex)) {
    val vertexIndex = pair(0)
    val peerIndex = pair(1)
    val localIndexOfVertex = config.localIndexOfVertex(edgeIndex, vertexIndex)
    val localIndexOfPeer = config.localIndexOfVertex(edgeIndex, peerIndex)
    io.edgeFeeds(localIndexOfVertex).updateIsTight := updateIsTight
    io.edgeFeeds(localIndexOfVertex).updatePeerNode := io.vertexIns(localIndexOfPeer).updateNode
    io.edgeFeeds(localIndexOfVertex).updatePeerRoot := io.vertexIns(localIndexOfPeer).updateRoot
    io.edgeFeeds(localIndexOfVertex).updatePeerSpeed := io.vertexIns(localIndexOfPeer).updateSpeed
  }
  updateValid := RegNext(executeValid) init False
  updateIsFindObstacle := RegNext(executeIsFindObstacle)
  updateIsReset := RegNext(executeIsReset)
  updateState := RegNext(executeState)
  updateGrown := RegNext(executeGrown)
  if (config.contextBits > 0) updateContextId := RegNext(executeContextId)
  pipelineIndex += 1

  // update stage

  writeValid := RegNext(updateValid) init False
  writeIsFindObstacle := RegNext(updateIsFindObstacle) init False
  writeIsReset := RegNext(updateIsReset) init False
  writeGrown := RegNext(updateGrown)
  writeState := Mux(writeIsReset, EdgePersistent.resetValue(config, edgeIndex), RegNext(updateState))
  if (config.contextBits > 0) writeContextId := RegNext(updateContextId)
  pipelineIndex += 1

  // write stage
  if (config.contextBits > 0) {
    ram.write(
      address = writeContextId,
      data = writeState,
      enable = writeValid
    )
  } else {
    when(writeValid) {
      register := writeState
    }
  }
  pipelineIndex += 1

  // also compute maxGrowth in the write stage
  val maxGrowth = Reg(UInt(config.weightBits bits))
  maxGrowth := config.LengthNone
  val isOverallSpeedPositive = Reg(Bool)
  isOverallSpeedPositive := (left.writeShadow.speed === Speed.Stay && right.writeShadow.speed === Speed.Grow) ||
    (left.writeShadow.speed === Speed.Grow && right.writeShadow.speed === Speed.Stay) ||
    (left.writeShadow.speed === Speed.Grow && right.writeShadow.speed === Speed.Grow)
  when(writeIsFindObstacle) {
    when(left.writeShadow.node =/= right.writeShadow.node) {
      val value1 = Mux(left.writeShadow.speed === Speed.Shrink, left.writeGrown, U(maxGrowth.maxValue))
      val value2 = Mux(right.writeShadow.speed === Speed.Shrink, right.writeGrown, U(maxGrowth.maxValue))
      val value3 = Mux(
        left.writeShadow.speed === Speed.Grow && right.writeShadow.speed === Speed.Shrink,
        writeState.weight - left.writeGrown,
        U(maxGrowth.maxValue)
      )
      val value4 = Mux(
        left.writeShadow.speed === Speed.Shrink && right.writeShadow.speed === Speed.Grow,
        writeState.weight - right.writeGrown,
        U(maxGrowth.maxValue)
      )
      val value5 = Mux(
        left.writeShadow.speed === Speed.Grow && right.writeShadow.speed === Speed.Grow,
        (writeState.weight - writeGrown) >> 1,
        U(maxGrowth.maxValue)
      )
      val value6 = Mux(
        (left.writeShadow.speed === Speed.Grow && right.writeShadow.speed === Speed.Stay) ||
          (left.writeShadow.speed === Speed.Stay && right.writeShadow.speed === Speed.Grow),
        writeState.weight - writeGrown,
        U(maxGrowth.maxValue)
      )
      maxGrowth := Vec(value1, value2, value3, value4, value5, value6).reduceBalancedTree((l, r) => Mux(l < r, l, r))
    }
  }
  reportIsFindObstacle := RegNext(writeIsFindObstacle) init False
  if (config.contextBits > 0) reportContextId := RegNext(writeContextId)
  reportLeftShadow := RegNext(left.writeShadow)
  reportRightShadow := RegNext(right.writeShadow)

  // report stage
  io.output.valid := reportIsFindObstacle
  io.output.obstacle.assignFromBits(
    Mux(
      maxGrowth === 0 && isOverallSpeedPositive,
      config.obstacleSpec
        .dynConflict(
          reportLeftShadow.node,
          reportRightShadow.node,
          reportLeftShadow.root,
          reportRightShadow.root,
          B(config.incidentVerticesOf(edgeIndex)(0)),
          B(config.incidentVerticesOf(edgeIndex)(1))
        ),
      config.obstacleSpec.dynNonZeroGrow(maxGrowth)
    )
  )
  if (config.contextBits > 0) io.output.contextId := writeContextId

  def pipelineStages = pipelineIndex
}

// sbt 'testOnly *EdgeTest'
class EdgeTest extends AnyFunSuite {

  test("construct an Edge") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Edge(config, 0))
  }

}
