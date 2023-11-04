package microblossom

import spinal.core._
import spinal.lib._
import microblossom._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite

// persistent state of a vertex
case class VertexPersistent(config: DualConfig) extends Bundle {
  val speed = Speed()
  val node = Bits(config.vertexBits bits)
  val root = Bits(config.vertexBits bits)
  val isVirtual = Bool
  val isDefect = Bool
  val grown = UInt(config.weightBits bits)
}

object VertexPersistent {
  def resetValue(config: DualConfig, vertexIndex: Int): VertexPersistent = {
    val reset = VertexPersistent(config)
    reset.speed := Speed.Stay
    reset.node := config.IndexNone
    reset.root := config.IndexNone
    reset.isVirtual := Bool(config.isVirtual(vertexIndex))
    reset.isDefect := False
    reset.grown := 0
    reset
  }
}

case class VertexFeed(config: DualConfig) extends Bundle {
  // execute stage
  val executeGrown = UInt(config.weightBits bits)
  // update stage
  val updateNode = Bits(config.vertexBits bits)
  val updateRoot = Bits(config.vertexBits bits)
  val updateSpeed = Speed()
  // write stage
  val writeShadow = VertexShadow(config)
  val writeGrown = UInt(config.weightBits bits)
}

case class VertexPropagator(config: DualConfig) extends Bundle {
  val valid = Bool
  val node = Bits(config.vertexBits bits)
  val root = Bits(config.vertexBits bits)
}

case class VertexShadow(config: DualConfig) extends Bundle {
  val speed = Speed()
  val node = Bits(config.vertexBits bits)
  val root = Bits(config.vertexBits bits)
}

case class Vertex(config: DualConfig, vertexIndex: Int) extends Component {
  val io = new Bundle {
    val input = in(BroadcastMessage(config))
    val vertexFeeds = out(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(VertexFeed(config)))
    val edgeIns = in(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(EdgeFeed(config)))
  }

  private var pipelineIndex = 0

  /*
   * pipeline input signals
   */

  val executeValid = Bool
  val executeState = VertexPersistent(config)
  val executeResult = VertexPersistent(config)
  val executeInstruction = Instruction(config)
  val executeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val updateValid = Bool
  val updateState = VertexPersistent(config)
  val updateResult = VertexPersistent(config)
  val updateResultShadow = VertexShadow(config)
  val updateInstruction = Instruction(config)
  val updateContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val writeValid = Bool
  val writeState = VertexPersistent(config)
  val writeShadow = VertexShadow(config)
  val writeInstruction = Instruction(config)
  val writeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  // fetch stage
  var ram: Mem[VertexPersistent] = null
  var register = Reg(VertexPersistent(config))
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(VertexPersistent(config), config.contextDepth)
    executeState := ram.readSync(
      address = io.input.contextId,
      enable = io.input.valid
    )
    executeContextId := RegNext(io.input.contextId)
  } else {
    executeState := RegNext(register)
  }
  executeValid := RegNext(io.input.valid) init False
  executeInstruction.assignFromBits(RegNext(io.input.instruction.asBits))
  pipelineIndex += 1

  // execute stage
  executeResult := executeState
  when(executeValid) {
    when(executeInstruction.isSetSpeed) {
      when(executeState.node === executeInstruction.field1) {
        executeResult.speed := executeInstruction.speed
      }
    }
    when(executeInstruction.isSetBlossom) {
      when(executeState.node === executeInstruction.field1 || executeState.root === executeInstruction.field1) {
        executeResult.node := executeInstruction.field2
        executeResult.speed := Speed.Grow
      }
    }
    when(executeInstruction.isGrow) {
      switch(executeState.speed.asUInt) {
        is(Speed.Grow) {
          executeResult.grown := (executeState.grown + executeInstruction.length)
        }
        is(Speed.Shrink) {
          executeResult.grown := (executeState.grown - executeInstruction.length)
        }
      }
    }
    if (config.supportAddDefectVertex) {
      when(executeInstruction.isAddDefect) {
        when(executeInstruction.field1 === vertexIndex) {
          executeResult.isDefect := True
          executeResult.speed := Speed.Grow
          assert(
            assertion = executeState.node === config.IndexNone,
            message = "Cannot set a vertex to defect when it's already occupied",
            severity = ERROR
          )
          executeResult.node := executeInstruction.extendedField2.resized
          executeResult.root := executeInstruction.extendedField2.resized
        }
      }
    }
  }
  for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
    val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
    io.vertexFeeds(localIndexOfEdge).executeGrown := executeResult.grown
  }

  updateValid := RegNext(executeValid) init False
  updateInstruction.assignFromBits(RegNext(executeInstruction.asBits))
  updateState := RegNext(executeResult)
  if (config.contextBits > 0) updateContextId := RegNext(executeContextId)
  pipelineIndex += 1

  // update stage
  updateResult := updateState
  updateResultShadow.node := updateState.node
  updateResultShadow.root := updateState.root
  updateResultShadow.speed := updateState.speed
  val propagators = Vec.fill(config.incidentEdgesOf(vertexIndex).length)(VertexPropagator(config))
  for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
    val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
    propagators(localIndexOfEdge).node := io.edgeIns(localIndexOfEdge).updatePeerNode
    propagators(localIndexOfEdge).root := io.edgeIns(localIndexOfEdge).updatePeerRoot
    propagators(localIndexOfEdge).valid := (
      io.edgeIns(localIndexOfEdge).updatePeerSpeed === Speed.Grow
    ) && (io.edgeIns(localIndexOfEdge).updateIsTight)
  }
  val selectedPropagator = propagators.reduceBalancedTree((l, r) => Mux(l.valid, l, r))
  when(updateValid) {
    when(executeInstruction.isGrow || executeInstruction.isSetSpeed || executeInstruction.isSetBlossom) {
      when(!updateState.isDefect && !updateState.isVirtual && updateState.grown === 0) {
        when(selectedPropagator.valid) {
          updateResult.node := selectedPropagator.node
          updateResult.root := selectedPropagator.root
          updateResult.speed := Speed.Grow
        } otherwise {
          updateResult.node := config.IndexNone
          updateResult.root := config.IndexNone
          updateResult.speed := Speed.Stay
        }
      }
    }
    when(executeInstruction.isFindObstacle) {
      when(updateState.speed === Speed.Shrink && updateState.grown === 0) {
        when(selectedPropagator.valid) {
          updateResultShadow.node := selectedPropagator.node
          updateResultShadow.root := selectedPropagator.root
          updateResultShadow.speed := Speed.Grow
        }
      }
    }
  }
  for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
    val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
    io.vertexFeeds(localIndexOfEdge).updateNode := updateState.node
    io.vertexFeeds(localIndexOfEdge).updateRoot := updateState.root
    io.vertexFeeds(localIndexOfEdge).updateSpeed := updateState.speed
  }

  writeValid := RegNext(updateValid) init False
  writeInstruction.assignFromBits(RegNext(updateInstruction.asBits))
  writeState := Mux(updateInstruction.isReset, VertexPersistent.resetValue(config, vertexIndex), RegNext(updateResult))
  writeShadow := RegNext(updateResultShadow)
  if (config.contextBits > 0) writeContextId := RegNext(updateContextId)
  pipelineIndex += 1

  // write stage
  for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
    val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
    io.vertexFeeds(localIndexOfEdge).writeShadow := writeShadow
    io.vertexFeeds(localIndexOfEdge).writeGrown := writeState.grown
  }

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

  def pipelineStages = pipelineIndex
}

// sbt 'testOnly *VertexTest'
class VertexTest extends AnyFunSuite {

  test("construct a Vertex") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Vertex(config, 0))
  }

  test("test pipeline registers") {
    // gtkwave simWorkspace/Vertex/testA.fst
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json", minimizeBits = false)
    val instructionSpec = InstructionSpec(config)
    config.sanityCheck()
    Config.sim
      .compile({
        val dut = Vertex(config, 0)
        dut.executeValid.simPublic()
        dut.executeInstruction.simPublic()
        dut.updateValid.simPublic()
        dut.updateInstruction.simPublic()
        dut.writeValid.simPublic()
        dut.writeInstruction.simPublic()
        dut.register.simPublic()
        dut
      })
      .doSim("testA") { dut =>
        dut.io.input.valid #= false
        dut.io.input.instruction #= 0
        dut.clockDomain.forkStimulus(period = 10)
        dut.clockDomain.waitSampling()

        dut.register.speed #= Speed.Stay
        val nodeIndex = 6
        dut.register.node #= nodeIndex
        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.clockDomain.waitSampling()
        dut.io.input.valid #= true
        val setSpeedInstruction = instructionSpec.generateSetSpeed(nodeIndex, Speed.Grow)
        dut.io.input.instruction #= setSpeedInstruction

        dut.clockDomain.waitSampling()
        dut.io.input.valid #= false
        dut.io.input.instruction #= 0
        sleep(1)
        assert(dut.io.input.valid.toBoolean == false)
        assert(dut.io.input.instruction.toLong == 0)
        assert(dut.executeValid.toBoolean == true)
        assert(dut.executeInstruction.toLong == setSpeedInstruction)
        assert(dut.updateValid.toBoolean == false)
        assert(dut.updateInstruction.toLong == 0)
        assert(dut.writeValid.toBoolean == false)
        assert(dut.writeInstruction.toLong == 0)
        // assert(dut.executeInstruction.toLong == setSpeedInstruction)
        // assert(dut.updateInstruction.toLong == 0)
        // assert(dut.writeInstruction.toLong == 0)

        dut.clockDomain.waitSampling()
        sleep(1)
        assert(dut.io.input.valid.toBoolean == false)
        assert(dut.io.input.instruction.toLong == 0)
        assert(dut.executeValid.toBoolean == false)
        assert(dut.executeInstruction.toLong == 0)
        assert(dut.updateValid.toBoolean == true)
        assert(dut.updateInstruction.toLong == setSpeedInstruction)
        assert(dut.writeValid.toBoolean == false)
        assert(dut.writeInstruction.toLong == 0)

        dut.clockDomain.waitSampling()
        sleep(1)
        assert(dut.io.input.valid.toBoolean == false)
        assert(dut.io.input.instruction.toLong == 0)
        assert(dut.executeValid.toBoolean == false)
        assert(dut.executeInstruction.toLong == 0)
        assert(dut.updateValid.toBoolean == false)
        assert(dut.updateInstruction.toLong == 0)
        assert(dut.writeValid.toBoolean == true)
        assert(dut.writeInstruction.toLong == setSpeedInstruction)

        dut.clockDomain.waitSampling()
        sleep(1)
        assert(dut.io.input.valid.toBoolean == false)
        assert(dut.io.input.instruction.toLong == 0)
        assert(dut.executeValid.toBoolean == false)
        assert(dut.executeInstruction.toLong == 0)
        assert(dut.updateValid.toBoolean == false)
        assert(dut.updateInstruction.toLong == 0)
        assert(dut.writeValid.toBoolean == false)
        assert(dut.writeInstruction.toLong == 0)
        assert(dut.register.speed.toInt == Speed.Grow)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
      }
  }

}
