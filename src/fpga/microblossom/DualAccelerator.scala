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

case class DualAccelerator(config: DualConfig) extends Component {
  val io = new Bundle {
    val valid = in(Bool)
    val instruction = in(Instruction())
    val contextId = (config.contextBits > 0) generate (in UInt (config.contextBits bits))
    val state = out(DualAcceleratorState())
  }

  val instructionReg = Instruction()
  instructionReg.assignFromBits(RegNext(io.instruction.asBits))

  io.state := DualAcceleratorState.Normal

  val broadcastMessage = Instruction(config)

  broadcastMessage.widthConvertedFrom(io.instruction)
  val broadcastInstruction = Instruction(config)
  broadcastInstruction.assignFromBits(Delay(RegNext(broadcastMessage.asBits), config.broadcastDelay))

  // instantiate vertices and edges
  val vertices = Seq
    .range(0, config.vertexNum)
    .map(vertexIndex => new Vertex(config, vertexIndex))

  vertices.foreach(vertex => {
    vertex.io.instruction := broadcastInstruction
    // vertex.io.valid :=
  })

  val edges = Seq
    .range(0, config.edgeNum)
    .map(edgeIndex => new Edge(config, edgeIndex))

  edges.foreach(edge => {
    edge.io.instruction := broadcastInstruction
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
    // val config = DualConfig(filename = "./resources/graphs/code_capacity_d3.json")
    // val config = DualConfig(filename = "./resources/graphs/example_code_capacity_planar_d5.json")
    val config = DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json")
    Config.spinal.generateVerilog(DualAccelerator(config))
  }

  test("test pipeline registers") {
    // gtkwave simWorkspace/DualAccelerator/testA.fst
    val config = DualConfig(filename = "./resources/graphs/code_capacity_d3.json", minimizeBits = false)
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
        dut.io.valid #= false
        dut.clockDomain.forkStimulus(period = 10)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.clockDomain.waitSampling()
        dut.io.valid #= true
        dut.io.instruction #= dut.config.instructionSpec.generateReset()

        dut.clockDomain.waitSampling()
        dut.io.valid #= false

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
      }
  }
}
