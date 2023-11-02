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

  // width conversion
  val broadcastMessage = BroadcastMessage(config)
  broadcastMessage.instruction.widthConvertedFrom(io.input.instruction)
  broadcastMessage.valid := io.input.valid
  if (config.contextBits > 0) { broadcastMessage.contextId := io.input.contextId }

  // delay the signal so that the synthesizer can automatically balancing the registers
  val broadcastRegInserted = Delay(RegNext(broadcastMessage), config.broadcastDelay)

  // instantiate vertices and edges
  val vertices = Seq
    .range(0, config.vertexNum)
    .map(vertexIndex => new Vertex(config, vertexIndex))

  vertices.foreach(vertex => {
    vertex.io.input := broadcastRegInserted
  })

  val edges = Seq
    .range(0, config.edgeNum)
    .map(edgeIndex => new Edge(config, edgeIndex))

  val edgeOutputs = Vec.fill(config.edgeNum)(ConvergecastMessage(config))
  edges.foreach(edge => {
    edge.io.input := broadcastRegInserted
    edgeOutputs(edge.edgeIndex) := edge.io.output
  })

  // connect the vertices and edges
  for (vertexIndex <- Range(0, config.vertexNum)) {
    val vertex = vertices(vertexIndex)
    for (edgeIndex <- config.incidentEdgesOf(vertexIndex)) {
      val edge = edges(edgeIndex)
      val localIndexOfVertex = config.localIndexOfVertex(edgeIndex, vertexIndex)
      val localIndexOfEdge = config.localIndexOfEdge(vertexIndex, edgeIndex)
      vertex.io.vertexFeeds(localIndexOfEdge) <> edge.io.vertexIns(localIndexOfVertex)
      vertex.io.edgeIns(localIndexOfEdge) <> edge.io.edgeFeeds(localIndexOfVertex)
    }
  }

  // gather the results in a tree structure
  val edgeOutput = ConvergecastMessage(config)
  edgeOutput := edgeOutputs.reduceBalancedTree((left, right) => {
    Mux(
      left.obstacle.isConflict,
      left,
      Mux(
        right.obstacle.isConflict,
        right, {
          assert(
            assertion = left.obstacle.isNonZeroGrow && right.obstacle.isNonZeroGrow,
            message = "simple reduce function does not consider more obstacles",
            severity = ERROR
          )
          Mux(left.obstacle.length < right.obstacle.length, left, right)
        }
      )
    )
  })

  // delay the signal so that the synthesizer can automatically balancing the registers
  val edgeOutputRegInserted = Delay(RegNext(edgeOutput), config.convergecastDelay)

  // width conversion
  io.output.obstacle.widthConvertedFrom(edgeOutputRegInserted.obstacle)
  io.output.valid := edgeOutputRegInserted.valid
  if (config.contextBits > 0) { io.output.contextId := edgeOutputRegInserted.contextId }

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
