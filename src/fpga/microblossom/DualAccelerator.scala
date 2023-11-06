package microblossom

import scala.collection.mutable.ArrayBuffer
import scala.collection.mutable.Map
import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import spinal.core._
import spinal.lib._
import util._
import spinal.core.sim._
import org.scalatest.funsuite.AnyFunSuite
import scala.util.control.Breaks._

object DualAcceleratorState extends SpinalEnum {
  val Normal, Busy, InstructionError = newElement()
}

case class BroadcastMessage(config: DualConfig) extends Bundle {
  val valid = Bool
  val instruction = Instruction(config)
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

case class ConvergecastMessage(config: DualConfig) extends Bundle {
  val valid = Bool
  val obstacle = Obstacle(config)
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
}

case class DualAccelerator(config: DualConfig, ioConfig: DualConfig = DualConfig()) extends Component {
  val io = new Bundle {
    val input = in(BroadcastMessage(ioConfig))
    val output = out(ConvergecastMessage(ioConfig))
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
          // assert(
          //   assertion = left.obstacle.isNonZeroGrow && right.obstacle.isNonZeroGrow,
          //   message = "simple reduce function does not consider more obstacles",
          //   severity = ERROR
          // )
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

  def simExecute(instruction: Long): BigInt = {
    io.input.valid #= true
    io.input.instruction #= instruction
    clockDomain.waitSampling()
    io.input.valid #= false
    for (idx <- 0 to (config.readLatency - 1)) { clockDomain.waitSampling() }
    sleep(1)
    io.output.obstacle.toBigInt
  }

  // a temporary solution without primal offloading
  def simFindObstacle(maxGrowth: Long): (BigInt, Long) = {
    var obstacle = simExecute(ioConfig.instructionSpec.generateFindObstacle())
    var reader = ObstacleReader(ioConfig, obstacle)
    var grown = 0.toLong
    breakable {
      while (reader.rspCode == RspCode.NonZeroGrow) {
        var length = reader.length.toLong
        if (length == ioConfig.LengthNone) {
          break
        }
        if (length + grown > maxGrowth) {
          length = maxGrowth - grown
        }
        if (length == 0) {
          obstacle = config.obstacleSpec.generateNonZeroGrow(0)
          break
        }
        grown += length
        simExecute(ioConfig.instructionSpec.generateGrow(length))
        obstacle = simExecute(ioConfig.instructionSpec.generateFindObstacle())
        reader = ObstacleReader(ioConfig, obstacle)
      }
    }
    (obstacle, grown)
  }

  // before compiling the simulator, mark the fields as public to enable snapshot
  def simMakePublicSnapshot() = {
    vertices.foreach(vertex => {
      vertex.register.simPublic()
      vertex.io.simPublic()
    })
    edges.foreach(edge => {
      edge.register.simPublic()
      edge.io.simPublic()
    })
  }

  // take a snapshot of the dual module, in the format of fusion blossom visualization
  def simSnapshot(abbrev: Boolean = true): Json = {
    // https://circe.github.io/circe/api/io/circe/JsonObject.html
    var jsonVertices = ArrayBuffer[Json]()
    vertices.foreach(vertex => {
      val register = vertex.register
      val vertexMap = Map(
        (if (abbrev) { "v" }
         else { "is_virtual" }) -> Json.fromBoolean(register.isVirtual.toBoolean),
        (if (abbrev) { "s" }
         else { "is_defect" }) -> Json.fromBoolean(register.isDefect.toBoolean)
      )
      val node = register.node.toLong
      if (node != config.IndexNone) {
        vertexMap += ((
          if (abbrev) { "p" }
          else { "propagated_dual_node" },
          Json.fromLong(node)
        ))
      }
      val root = register.root.toLong
      if (root != config.IndexNone) {
        vertexMap += ((
          if (abbrev) { "pg" }
          else { "propagated_grandson_dual_node" },
          Json.fromLong(root)
        ))
      }
      jsonVertices.append(Json.fromFields(vertexMap))
    })
    var jsonEdges = ArrayBuffer[Json]()
    edges.foreach(edge => {
      val register = edge.register
      val neighbors = config.incidentVerticesOf(edge.edgeIndex)
      val leftReg = vertices(neighbors(0)).register
      val rightReg = vertices(neighbors(1)).register
      val edgeMap = Map(
        (if (abbrev) { "w" }
         else { "weight" }) -> Json.fromLong(register.weight.toLong),
        (if (abbrev) { "l" }
         else { "left" }) -> Json.fromLong(neighbors(0)),
        (if (abbrev) { "r" }
         else { "right" }) -> Json.fromLong(neighbors(1)),
        (if (abbrev) { "lg" }
         else { "left_growth" }) -> Json.fromLong(leftReg.grown.toLong),
        (if (abbrev) { "rg" }
         else { "right_growth" }) -> Json.fromLong(rightReg.grown.toLong)
      )
      val leftNode = leftReg.node.toLong
      if (leftNode != config.IndexNone) {
        edgeMap += ((
          if (abbrev) { "ld" }
          else { "left_dual_node" },
          Json.fromLong(leftNode)
        ))
      }
      val leftRoot = leftReg.root.toLong
      if (leftRoot != config.IndexNone) {
        edgeMap += ((
          if (abbrev) { "lgd" }
          else { "left_grandson_dual_node" },
          Json.fromLong(leftRoot)
        ))
      }
      val rightNode = rightReg.node.toLong
      if (rightNode != config.IndexNone) {
        edgeMap += ((
          if (abbrev) { "rd" }
          else { "right_dual_node" },
          Json.fromLong(rightNode)
        ))
      }
      val rightRoot = rightReg.root.toLong
      if (rightRoot != config.IndexNone) {
        edgeMap += ((
          if (abbrev) { "rgd" }
          else { "right_grandson_dual_node" },
          Json.fromLong(rightRoot)
        ))
      }
      jsonEdges.append(Json.fromFields(edgeMap))
    })
    Json.fromFields(
      Map(
        "vertices" -> Json.fromValues(jsonVertices),
        "edges" -> Json.fromValues(jsonEdges)
      )
    )
  }

}

// sbt 'testOnly *DualAcceleratorTest'
class DualAcceleratorTest extends AnyFunSuite {

  test("construct accelerator from file") {
    // val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_planar_d5.json")
    // val config = DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json")
    Config.spinal().generateVerilog(DualAccelerator(config))
  }

  test("test pipeline registers") {
    // gtkwave simWorkspace/DualAccelerator/testA.fst
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json", minimizeBits = false)
    config.sanityCheck()
    Config.sim
      .compile({
        val dut = DualAccelerator(config)
        dut.simMakePublicSnapshot()
        dut
      })
      .doSim("testA") { dut =>
        val ioConfig = dut.ioConfig
        dut.io.input.valid #= false
        dut.clockDomain.forkStimulus(period = 10)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.simExecute(ioConfig.instructionSpec.generateReset())
        dut.simExecute(ioConfig.instructionSpec.generateAddDefect(0, 0))
        var obstacle = dut.simExecute(ioConfig.instructionSpec.generateFindObstacle())

        assert(obstacle == 100 << 2) // at most grow 100
        val reader = ObstacleReader(ioConfig, obstacle)
        assert(reader.rspCode == RspCode.NonZeroGrow)
        assert(reader.length == 100)

        dut.simExecute(ioConfig.instructionSpec.generateGrow(30))
        var obstacle2 = dut.simExecute(ioConfig.instructionSpec.generateFindObstacle())
        val reader2 = ObstacleReader(ioConfig, obstacle2)
        assert(reader2.rspCode == RspCode.NonZeroGrow)
        assert(reader2.length == 70)

        val (obstacle3, grown3) = dut.simFindObstacle(50)
        val reader3 = ObstacleReader(ioConfig, obstacle3)
        assert(grown3 == 50)
        assert(reader3.rspCode == RspCode.NonZeroGrow)
        assert(reader3.length == 0)

        val (obstacle4, grown4) = dut.simFindObstacle(1000)
        val reader4 = ObstacleReader(ioConfig, obstacle4)
        assert(grown4 == 20)
        assert(reader4.rspCode == RspCode.Conflict)
        assert(reader4.field1 == 0) // node1
        assert(reader4.field2 == ioConfig.IndexNone) // node2 (here it's virtual)
        assert(reader4.field3 == 0) // touch1
        assert(reader4.field4 == ioConfig.IndexNone) // touch2 (here it's virtual)
        assert(reader4.field5 == 0) // vertex1
        assert(reader4.field6 == 3) // vertex2

        println(dut.simSnapshot().noSpacesSortKeys)
      }
  }

}

// sbt "runMain microblossom.DualAcceleratorDebug1"
object DualAcceleratorDebug1 extends App {
  // gtkwave simWorkspace/DualAccelerator/testB.fst
  val config = DualConfig(filename = "./resources/graphs/example_code_capacity_planar_d3.json", minimizeBits = false)
  config.sanityCheck()
  Config.sim
    .compile({
      val dut = DualAccelerator(config)
      dut.simMakePublicSnapshot()
      dut
    })
    .doSim("testB") { dut =>
      val ioConfig = dut.ioConfig
      dut.io.input.valid #= false
      dut.clockDomain.forkStimulus(period = 10)

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

      dut.simExecute(ioConfig.instructionSpec.generateReset())
      dut.simExecute(ioConfig.instructionSpec.generateAddDefect(0, 0))
      dut.simExecute(ioConfig.instructionSpec.generateAddDefect(4, 1))
      dut.simExecute(ioConfig.instructionSpec.generateAddDefect(8, 2))

      val (obstacle, grown) = dut.simFindObstacle(1000)
      assert(grown == 50)
      val reader = ObstacleReader(ioConfig, obstacle)
      assert(reader.rspCode == RspCode.Conflict)
      println(reader.field1, reader.field2, reader.field3, reader.field4)

      println(dut.simSnapshot().noSpacesSortKeys)

      dut.simExecute(ioConfig.instructionSpec.generateSetBlossom(0, 3000))
      dut.simExecute(ioConfig.instructionSpec.generateSetBlossom(1, 3000))
      dut.simExecute(ioConfig.instructionSpec.generateSetBlossom(2, 3000))
      dut.simExecute(ioConfig.instructionSpec.generateSetSpeed(3, Speed.Shrink))

      for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

    }
}

// sbt "runMain microblossom.DualAcceleratorExamples"
object DualAcceleratorExamples extends App {
  for (d <- Seq(3, 5, 7)) {
    val config =
      DualConfig(filename = "./resources/graphs/example_code_capacity_planar_d%d.json".format(d), minimizeBits = true)
    Config.spinal("gen/example_code_capacity_planar_d%d".format(d)).generateVerilog(DualAccelerator(config))
  }
  for (d <- Seq(3, 5, 7)) {
    val config =
      DualConfig(filename = "./resources/graphs/example_code_capacity_rotated_d%d.json".format(d), minimizeBits = true)
    Config.spinal("gen/example_code_capacity_rotated_d%d".format(d)).generateVerilog(DualAccelerator(config))
  }
  for (d <- Seq(3, 5, 7, 9, 11)) {
    val config =
      DualConfig(
        filename = "./resources/graphs/example_phenomenological_rotated_d%d.json".format(d),
        minimizeBits = true
      )
    Config.spinal("gen/example_phenomenological_rotated_d%d".format(d)).generateVerilog(DualAccelerator(config))
  }
}
