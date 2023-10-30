package microblossom

import spinal.core._
import microblossom._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite

// persistent state of an edge
case class EdgePersistent(config: DualConfig) extends Bundle {
  val weight = Bits(config.weightBits bits)
}

case class EdgeOutput(config: DualConfig) extends Bundle {
  // execute stage

  // update stage
  val updateIsTight = Bool()
  // write stage

}

case class Edge(config: DualConfig, edgeIndex: Int) extends Component {
  val io = new Bundle {
    val valid = in Bool ()
    val instruction = in(Instruction(config))
    val contextId = (config.contextBits > 0) generate (in UInt (config.contextBits bits))
    val edgeOutputs = out(Vec.fill(2)(EdgeOutput(config)))
    val vertexInputs = in(Vec.fill(2)(VertexOutput(config)))
  }

  private var pipelineIndex = 0;

  /*
   * pipeline input signals
   */

  val executeValid = Bool
  val executeState = EdgePersistent(config)
  val executeIsFindObstacle = Bool()
  val executeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val updateValid = Bool
  val updateState = EdgePersistent(config)
  val updateIsFindObstacle = Bool()
  val updateContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val writeValid = Bool
  val writeState = EdgePersistent(config)
  val writeIsFindObstacle = Bool()
  val writeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  // fetch stage
  var ram: Mem[EdgePersistent] = null
  var register = Reg(EdgePersistent(config))
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(EdgePersistent(config), config.contextDepth)
    executeState := ram.readSync(
      address = io.contextId,
      enable = io.valid
    )
    executeContextId := RegNext(io.contextId)
  } else {
    executeState := RegNext(register)
  }
  executeValid := RegNext(io.valid) init False
  executeIsFindObstacle := RegNext(io.instruction.isFindObstacle)
  pipelineIndex += 1;

  // execute stage

  val updateIsTight = RegNext(
    io.vertexInputs(0).executeGrown.asUInt + io.vertexInputs(1).executeGrown.asUInt >= executeState.weight.asUInt
  )
  for (vertexIndex <- config.incidentVerticesOf(edgeIndex)) {
    val localIndexOfVertex = config.localIndexOfVertex(edgeIndex, vertexIndex)
    io.edgeOutputs(localIndexOfVertex).updateIsTight := updateIsTight
  }
  updateValid := RegNext(executeValid) init False
  updateIsFindObstacle := RegNext(executeIsFindObstacle)
  updateState := RegNext(executeState)
  if (config.contextBits > 0) updateContextId := RegNext(executeContextId)
  pipelineIndex += 1;

  // update stage

  writeValid := RegNext(updateValid) init False
  writeIsFindObstacle := RegNext(updateIsFindObstacle)
  writeState := RegNext(updateState)
  if (config.contextBits > 0) writeContextId := RegNext(updateContextId)
  pipelineIndex += 1;

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
  pipelineIndex += 1;

  // also generate response in write stage

  def pipelineStages = pipelineIndex
}

// sbt 'testOnly *EdgeTest'
class EdgeTest extends AnyFunSuite {

  test("construct an Edge") {
    val config = DualConfig(filename = "./resources/graphs/example_repetition_code.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal.generateVerilog(Edge(config, 0))
  }

}
