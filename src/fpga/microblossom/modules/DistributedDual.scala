package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite

case class DistributedDual(config: DualConfig, ioConfig: DualConfig = DualConfig()) extends Component {
  val io = new Bundle {
    val message = in(BroadcastMessage(ioConfig))

    val maxLength = out(ConvergecastMaxLength(ioConfig.weightBits))
    val conflict = out(ConvergecastConflict(ioConfig.vertexBits))
  }

  // width conversion
  val broadcastMessage = BroadcastMessage(config)
  broadcastMessage.instruction.widthConvertedFrom(io.message.instruction)
  broadcastMessage.valid := io.message.valid
  if (config.contextBits > 0) { broadcastMessage.contextId := io.message.contextId }

  // delay the signal so that the synthesizer can automatically balancing the registers
  val broadcastRegInserted = Delay(RegNext(broadcastMessage), config.broadcastDelay)

  // instantiate vertices, edges and offloaders
  val vertices = Seq
    .range(0, config.vertexNum)
    .map(vertexIndex => new Vertex(config, vertexIndex))
  val edges = Seq
    .range(0, config.edgeNum)
    .map(edgeIndex => new Edge(config, edgeIndex))
  val offloaders = Seq
    .range(0, config.offloaderNum)
    .map(offloaderIndex => new Offloader(config, offloaderIndex))

  // connect vertex I/O
  for ((vertex, vertexIndex) <- vertices.zipWithIndex) {
    vertex.io.message := broadcastRegInserted
    for ((edgeIndex, localIndex) <- config.incidentEdgesOf(vertexIndex).zipWithIndex) {
      vertex.io.edgeInputs(localIndex) := edges(edgeIndex).io.stageOutputs
    }
    for ((offloaderIndex, localIndex) <- config.incidentOffloaderOf(vertexIndex).zipWithIndex) {
      vertex.io.offloaderInputs(localIndex) := offloaders(offloaderIndex).io.stageOutputs
    }
    for ((edgeIndex, localIndex) <- config.incidentEdgesOf(vertexIndex).zipWithIndex) {
      vertex.io.peerVertexInputsExecute3(localIndex) := vertices(vertexIndex).io.stageOutputs.executeGet3
    }
  }

  // connect edge I/O
  for ((edge, edgeIndex) <- edges.zipWithIndex) {
    edge.io.message := broadcastRegInserted
    val (leftVertex, rightVertex) = config.incidentVerticesOf(edgeIndex)
    edge.io.leftVertexInput := vertices(leftVertex).io.stageOutputs
    edge.io.rightVertexInput := vertices(rightVertex).io.stageOutputs
  }

  // connect offloader I/O
  for ((offloader, offloaderIndex) <- offloaders.zipWithIndex) {
    for ((vertexIndex, localIndex) <- config.offloaderNeighborVertexIndices(offloaderIndex).zipWithIndex) {
      offloader.io.vertexInputsOffloadGet3(localIndex) := vertices(vertexIndex).io.stageOutputs.offloadGet3
    }
    for ((edgeIndex, localIndex) <- config.offloaderNeighborEdgeIndices(offloaderIndex).zipWithIndex) {
      offloader.io.neighborEdgeInputsOffloadGet3(localIndex) := edges(edgeIndex).io.stageOutputs.offloadGet3
    }
    val edgeIndex = config.offloaderEdgeIndex(offloaderIndex)
    offloader.io.edgeInputOffloadGet3 := edges(edgeIndex).io.stageOutputs.offloadGet3
  }

  // build convergecast tree for maxLength
  val maxLengthConvergcastTree =
    Vec.fill(config.graph.vertex_edge_binary_tree.nodes.length)(ConvergecastMaxLength(config.weightBits))
  for ((treeNode, index) <- config.graph.vertex_edge_binary_tree.nodes.zipWithIndex) {
    if (index < config.vertexNum) {
      val vertexIndex = index
      maxLengthConvergcastTree(index) := vertices(vertexIndex).io.maxLength
    } else if (index < config.vertexNum + config.edgeNum) {
      val edgeIndex = index - config.vertexNum
      maxLengthConvergcastTree(index) := edges(edgeIndex).io.maxLength
    } else {
      val left = maxLengthConvergcastTree(treeNode.l.get.toInt)
      val right = maxLengthConvergcastTree(treeNode.r.get.toInt)
      when(left.length < right.length) {
        maxLengthConvergcastTree(index) := left
      } otherwise {
        maxLengthConvergcastTree(index) := right
      }
    }
  }
  io.maxLength := maxLengthConvergcastTree(config.graph.vertex_edge_binary_tree.nodes.length - 1).resized

  // build convergecast tree of conflict
  val conflictConvergecastTree =
    Vec.fill(config.graph.edge_binary_tree.nodes.length)(ConvergecastConflict(config.vertexBits))
  for ((treeNode, index) <- config.graph.edge_binary_tree.nodes.zipWithIndex) {
    if (index < config.edgeNum) {
      val edgeIndex = index
      conflictConvergecastTree(index) := edges(edgeIndex).io.conflict
    } else {
      val left = conflictConvergecastTree(treeNode.l.get.toInt)
      val right = conflictConvergecastTree(treeNode.r.get.toInt)
      when(left.valid) {
        conflictConvergecastTree(index) := left
      } otherwise {
        conflictConvergecastTree(index) := right
      }
    }
  }
  val convergecastedConflict = conflictConvergecastTree(config.graph.edge_binary_tree.nodes.length - 1)
  io.conflict.valid := convergecastedConflict.valid
  io.conflict.node1 := convergecastedConflict.node1.resized
  io.conflict.node2 := convergecastedConflict.node2.resized
  io.conflict.touch1 := convergecastedConflict.touch1.resized
  io.conflict.touch2 := convergecastedConflict.touch2.resized
  io.conflict.vertex1 := convergecastedConflict.vertex1.resized
  io.conflict.vertex2 := convergecastedConflict.vertex2.resized

}

// sbt 'testOnly microblossom.modules.DistributedDualTest'
class DistributedDualTest extends AnyFunSuite {

  test("construct a DistributedDual") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(DistributedDual(config))
  }

}

// sbt 'testOnly microblossom.modules.DistributedDualEstimation'
class DistributedDualEstimation extends AnyFunSuite {

  test("logic delay") {
    def dualConfig(name: String, removeWeight: Boolean = false): DualConfig = {
      val config = DualConfig(filename = s"./resources/graphs/example_$name.json")
      if (removeWeight) {
        for (edgeIndex <- 0 until config.edgeNum) {
          config.graph.weighted_edges(edgeIndex).w = 2
        }
        config.fitGraph()
        require(config.weightBits == 2)
      }
      config
    }

    val configurations = List(
      // TODO: estimate resource usage after correctness verification
      // synth: 387, impl: 310 Slice LUTs (0.14% on ZCU106)
      (dualConfig("code_capacity_d5"), "code capacity repetition d=5"),
      // synth: 1589, impl: 1309 Slice LUTs (0.6% on ZCU106)
      (dualConfig("code_capacity_rotated_d5"), "code capacity rotated d=5"),
      // synth: 15470, impl: 13186 Slice LUTs (6.03% on ZCU106)
      (dualConfig("phenomenological_rotated_d5"), "phenomenological d=5"),
      // synth: 31637, impl: 25798 Slice LUTs (11.80% on ZCU106)
      (dualConfig("circuit_level_d5", true), "circuit-level d=5 (unweighted)"),
      // synth: 41523, impl: 34045 Slice LUTs (15.57% on ZCU106)
      (dualConfig("circuit_level_d5"), "circuit-level d=5"),
      // synth: 299282, impl:
      (dualConfig("circuit_level_d9"), "circuit-level d=9")
    )
    for ((config, name) <- configurations) {
      // val reports = Vivado.report(DistributedDual(config))
      val reports = Vivado.report(DistributedDual(config), useImpl = true)
      println(s"$name:")
      reports.resource.primitivesTable.print()
    }
  }

}
