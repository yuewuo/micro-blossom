package microblossom

import spinal.core._
import microblossom._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite

// persistent state of a vertex
case class VertexPersistent(config: DualConfig) extends Bundle {
  val speed = Speed()
  val node = Bits(config.vertexBits bits)
  val root = Bits(config.vertexBits bits)
  val isVirtual = Bool()
  val isDefect = Bool()
}

case class VertexOutput(config: DualConfig) extends Bundle {
  // fetch stage

  // execute stage

  // update stage

  // write stage
}

case class Vertex2() extends Component {
  val io = new Bundle {
    val opcode = out(Bits(2 bits))
  }
}

case class Vertex(config: DualConfig, vertexIndex: Int) extends Component {
  // printf("hello\n");
  val io = new Bundle {
    val valid = in Bool ()
    val instruction = in(Instruction(config))
    val contextId = (config.contextBits > 0) generate (in UInt (config.contextBits bits))
    // val vertexOutputs = out(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(VertexOutput(config)))
    // val edgeInputs = in(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(EdgeOutput(config)))
  }

  private var pipelineIndex = 0;

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
  val updateInstruction = Instruction(config)
  val updateContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  val writeValid = Bool
  val writeState = VertexPersistent(config)
  val writeInstruction = Instruction(config)
  val writeContextId = (config.contextBits > 0) generate UInt(config.contextBits bits)

  // fetch stage (optional)
  var ram: Mem[VertexPersistent] = null
  var register = Reg(VertexPersistent(config))
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(VertexPersistent(config), config.contextDepth)
    executeState := ram.readSync(
      address = io.contextId,
      enable = io.valid
    )
    executeContextId := RegNext(io.contextId)
  } else {
    executeState := RegNext(register)
  }
  executeValid := RegNext(io.valid) init False
  executeInstruction.assignFromBits(RegNext(io.instruction.asBits))
  pipelineIndex += 1;

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
    when(executeInstruction.isGrow) {}
  }

  updateValid := RegNext(executeValid) init False
  updateInstruction.assignFromBits(RegNext(executeInstruction.asBits))
  updateState := RegNext(executeResult)
  if (config.contextBits > 0) updateContextId := RegNext(executeContextId)
  pipelineIndex += 1;

  // update stage

  writeValid := RegNext(updateValid) init False
  writeInstruction.assignFromBits(RegNext(updateInstruction.asBits))
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

  // there are 4 stages: fetch, execute, update, write

  def pipelineStages = pipelineIndex
}

// sbt 'testOnly *VertexTest'
class VertexTest extends AnyFunSuite {

  test("construct a Vertex") {
    val config = DualConfig(filename = "./resources/graphs/example_repetition_code.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal.generateVerilog(Vertex(config, 0))
  }

  test("test pipeline registers") {
    // gtkwave simWorkspace/Vertex/testA.fst
    val config = DualConfig(filename = "./resources/graphs/example_repetition_code.json", minimizeBits = false)
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
        dut.io.valid #= false
        dut.io.instruction #= 0
        dut.clockDomain.forkStimulus(period = 10)
        dut.clockDomain.waitSampling()

        dut.register.speed #= Speed.Stay
        val nodeIndex = 6
        dut.register.node #= nodeIndex
        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.clockDomain.waitSampling()
        dut.io.valid #= true
        val setSpeedInstruction = instructionSpec.generateSetSpeed(nodeIndex, Speed.Grow)
        dut.io.instruction #= setSpeedInstruction

        dut.clockDomain.waitSampling()
        dut.io.valid #= false
        dut.io.instruction #= 0
        sleep(1)
        assert(dut.io.valid.toBoolean == false)
        assert(dut.io.instruction.toLong == 0)
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
        assert(dut.io.valid.toBoolean == false)
        assert(dut.io.instruction.toLong == 0)
        assert(dut.executeValid.toBoolean == false)
        assert(dut.executeInstruction.toLong == 0)
        assert(dut.updateValid.toBoolean == true)
        assert(dut.updateInstruction.toLong == setSpeedInstruction)
        assert(dut.writeValid.toBoolean == false)
        assert(dut.writeInstruction.toLong == 0)

        dut.clockDomain.waitSampling()
        sleep(1)
        assert(dut.io.valid.toBoolean == false)
        assert(dut.io.instruction.toLong == 0)
        assert(dut.executeValid.toBoolean == false)
        assert(dut.executeInstruction.toLong == 0)
        assert(dut.updateValid.toBoolean == false)
        assert(dut.updateInstruction.toLong == 0)
        assert(dut.writeValid.toBoolean == true)
        assert(dut.writeInstruction.toLong == setSpeedInstruction)

        dut.clockDomain.waitSampling()
        sleep(1)
        assert(dut.io.valid.toBoolean == false)
        assert(dut.io.instruction.toLong == 0)
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
