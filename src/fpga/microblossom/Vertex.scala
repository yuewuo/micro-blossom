package microblossom

import spinal.core._
import microblossom._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite

// persistent state of a vertex
case class VertexPersistent(config: DualConfig) extends Bundle {
  val speed = CompactGrowState()
}

case class VertexOutput(config: DualConfig) extends Bundle {
  // fetch stage
  val speed = Bits(2 bits)
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
    val contextId = in UInt (config.contextBits bits)
    // val vertexOutputs = out(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(VertexOutput(config)))
    // val edgeInputs = in(Vec.fill(config.numIncidentEdgeOf(vertexIndex))(EdgeOutput(config)))
  }

  private var pipelineIndex = 0;

  /*
   * pipeline input signals
   */

  val executeValid = Bool()
  val executeState = VertexPersistent(config)
  val executeInstruction = Instruction(config)
  val executeContextId = UInt(config.contextBits bits)

  val updateValid = Bool()
  val updateState = VertexPersistent(config)
  val updateInstruction = Instruction(config)
  val updateContextId = UInt(config.contextBits bits)

  val writeValid = Bool()
  val writeState = VertexPersistent(config)
  val writeInstruction = Instruction(config)
  val writeContextId = UInt(config.contextBits bits)

  // fetch stage (optional)
  var ram: Mem[VertexPersistent] = null
  var register = VertexPersistent(config)
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(VertexPersistent(config), config.contextDepth)
    executeState := ram.readSync(
      address = io.contextId,
      enable = io.valid
    )
  } else {
    executeState := RegNextWhen(register, io.valid)
  }
  executeValid := RegNext(io.valid)
  executeInstruction := RegNextWhen(io.instruction, io.valid)
  executeContextId := RegNextWhen(io.contextId, io.valid)
  pipelineIndex += 1;

  // execute stage

  updateValid := RegNext(executeValid)
  updateState := RegNext(executeState)
  updateContextId := RegNext(executeContextId)
  pipelineIndex += 1;

  // update stage

  writeValid := RegNext(updateValid)
  writeState := RegNext(updateState)
  writeContextId := RegNext(updateContextId)
  pipelineIndex += 1;

  // write stage
  if (config.contextBits > 0) {
    ram.write(
      address = writeContextId,
      data = writeState,
      enable = writeValid
    )
    pipelineIndex += 1;
  } else {
    register := writeState
  }

  // there are 4 stages: fetch, execute, update, write

  def pipelineStages = pipelineIndex
}

// sbt 'testOnly *VertexTest'
class VertexTest extends AnyFunSuite {

  test("construct a Vertex") {
    val config = new DualConfig(filename = "./resources/graphs/example_repetition_code.json")
    config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    // config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal.generateVerilog(Vertex(config, 0))
  }

  test("no context switch") {
    // gtkwave simWorkspace/Vertex/testA.fst
    val config = new DualConfig(filename = "./resources/graphs/example_repetition_code.json")
    config.sanityCheck()
    Config.sim.compile(Vertex(config, 0)).doSim("testA") { dut =>
      dut.clockDomain.forkStimulus(period = 10)

      dut.io.valid #= false
      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

      dut.clockDomain.waitSampling()
      dut.io.valid #= true
      // dut.io.instruction #= Instruction(config).dut.clockDomain.waitSampling()

      dut.clockDomain.waitSampling()
      dut.io.valid #= false

      // dut.clockDomain.waitSampling()
      // dut.io.write #= false
      // dut.io.readAddress #= 0x33
      // sleep(1)
      // assert(dut.io.readValue.toInt == 0x1234)

      // dut.clockDomain.waitSampling()
      // dut.io.read #= false
      // sleep(1)
      // assert(dut.io.readValue.toInt == 0x5678)

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
    }
  }

}
