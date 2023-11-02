package microblossom

import spinal.core._
import spinal.lib._
import util._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite

object DualAcceleratorState extends SpinalEnum {
  val Normal, Busy, InstructionError = newElement()
}

case class BroadcastMessage(config: DualConfig) extends Bundle {
  val valid = Bool
  val instruction = Instruction(config)
  val contextId = (config.contextBits > 0) generate (in UInt (config.contextBits bits))
}

case class ConvergecastMessage(config: DualConfig) extends Bundle {
  val valid = Bool
  val obstacle = Obstacle(config)
  val contextId = (config.contextBits > 0) generate (in UInt (config.contextBits bits))
}

case class DualAccelerator(config: DualConfig, topConfig: DualConfig = DualConfig()) extends Component {
  val io = new Bundle {
    val input = in(BroadcastMessage(topConfig))
    val output = out(ConvergecastMessage(topConfig))
    val state = out(DualAcceleratorState())
  }

  io.state := DualAcceleratorState.Normal
  io.output.valid := False
  io.output.obstacle := 0
  if (config.contextBits > 0) {
    io.output.contextId := 0
  }

  // width conversion
  val broadcastMessage = BroadcastMessage(config)
  broadcastMessage.instruction.widthConvertedFrom(io.input.instruction)
  broadcastMessage.valid := io.input.valid
  if (config.contextBits > 0) {
    broadcastMessage.contextId := io.input.contextId
  }

  // delay the signal so that the synthesizer can automatically balancing the registers
  val broadcastRegInserted = BroadcastMessage(config)
  broadcastRegInserted.instruction.assignFromBits(
    Delay(RegNext(broadcastMessage.instruction.asBits), config.broadcastDelay)
  )
  broadcastRegInserted.valid := Delay(RegNext(io.input.valid), config.broadcastDelay)
  if (config.contextBits > 0) {
    broadcastRegInserted.contextId := Delay(
      RegNext(io.input.contextId),
      config.broadcastDelay
    )
  }

  // instantiate vertices and edges
  val vertices = Seq
    .range(0, config.vertexNum)
    .map(vertexIndex => new Vertex(config, vertexIndex))

  vertices.foreach(vertex => {
    vertex.io.instruction := broadcastRegInserted.instruction
    vertex.io.valid := broadcastRegInserted.valid
    if (config.contextBits > 0) { vertex.io.contextId := broadcastRegInserted.contextId }
  })

  val edges = Seq
    .range(0, config.edgeNum)
    .map(edgeIndex => new Edge(config, edgeIndex))

  edges.foreach(edge => {
    edge.io.instruction := broadcastRegInserted.instruction
    edge.io.valid := broadcastRegInserted.valid
    if (config.contextBits > 0) { edge.io.contextId := broadcastRegInserted.contextId }
  })

  // connect the vertices and edges
  for (vertexIndex <- Range(0, config.vertexNum)) {
    val vertex = vertices(vertexIndex)
    for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
      val edge = edges(edgeIndex)
      val localIndexOfVertex = config.localIndexOfVertex(edgeIndex, vertexIndex)
      val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
      vertex.io.vertexOutputs(localIndexOfEdge) <> edge.io.vertexInputs(localIndexOfVertex)
      vertex.io.edgeInputs(localIndexOfEdge) <> edge.io.edgeOutputs(localIndexOfVertex)
    }
  }

  // TODO: gather the results in a tree structure. tip: use reduceBalancedTree function
  // https://spinalhdl.github.io/SpinalDoc-RTD/master/SpinalHDL/Data%20types/Vec.html#lib-helper-functions
}

// sbt 'testOnly *DualAcceleratorTest'
class DualAcceleratorTest extends AnyFunSuite {

  test("construct accelerator from file") {
    // val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // val config = DualConfig(filename = "./resources/graphs/example_code_capacity_planar_d5.json")
    val config = DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json")
    Config.spinal.generateVerilog(DualAccelerator(config))
  }

  test("test pipeline registers") {
    // gtkwave simWorkspace/DualAccelerator/testA.fst
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json", minimizeBits = false)
    config.sanityCheck()
    Config.sim
      .compile({
        val dut = DualAccelerator(config)
        dut.vertices.foreach(vertex => {
          vertex.io.simPublic()
        })
        dut
      })
      .doSim("testA") { dut =>
        dut.io.input.valid #= false
        dut.clockDomain.forkStimulus(period = 10)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.clockDomain.waitSampling()
        dut.io.input.valid #= true
        dut.io.input.instruction #= dut.config.instructionSpec.generateReset()

        dut.clockDomain.waitSampling()
        dut.io.input.valid #= false

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
      }
  }
}
