package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import microblossom.combinatorial._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite

object Offloader {
  def getStages(
      config: DualConfig,
      offloaderIndex: Int
  ): Stages[Bundle, Bundle, Bundle, StageOffloadOffloader4, Bundle, Bundle, Bundle, Bundle, Bundle, Bundle] = {
    Stages(
      offload4 = () => StageOffloadOffloader4(config, offloaderIndex)
    )
  }
}

case class Offloader(config: DualConfig, offloaderIndex: Int, injectRegisters: Seq[String] = List()) extends Component {
  val io = new Bundle {
    val stageOutputs = out(Offloader.getStages(config, offloaderIndex).getStageOutput)
    val vertexInputsOffloadGet3 = in(
      Vec(
        for (vertexIndex <- config.offloaderNeighborVertexIndices(offloaderIndex))
          yield Vertex.getStages(config, vertexIndex).getStageOutput.offloadGet3
      )
    )
    val neighborEdgeInputsOffloadGet3 = in(
      Vec(
        for (edgeIndex <- config.offloaderNeighborEdgeIndices(offloaderIndex))
          yield Edge.getStages(config).getStageOutput.offloadGet3
      )
    )
    val edgeInputOffloadGet3 = in(Edge.getStages(config).getStageOutput.offloadGet3)
  }

  val stages = Offloader.getStages(config, offloaderIndex)
  stages.connectStageOutput(io.stageOutputs)

  def connectLogic(): Unit = {
    var offloader = config.graph.offloading(offloaderIndex)
    offloader.dm match {
      case Some(defectMatch) =>
        var offloadDefectMatch = OffloadDefectMatch()
        require(io.vertexInputsOffloadGet3.length == 2)
        offloadDefectMatch.io.edgeIsTight := io.edgeInputOffloadGet3.isTight
        offloadDefectMatch.io.leftIsDefect := io.vertexInputsOffloadGet3(0).state.isDefect
        offloadDefectMatch.io.leftSpeed := io.vertexInputsOffloadGet3(0).state.speed
        offloadDefectMatch.io.leftIsUniqueTight := io.vertexInputsOffloadGet3(0).isUniqueTight
        offloadDefectMatch.io.rightIsDefect := io.vertexInputsOffloadGet3(1).state.isDefect
        offloadDefectMatch.io.rightSpeed := io.vertexInputsOffloadGet3(1).state.speed
        offloadDefectMatch.io.rightIsUniqueTight := io.vertexInputsOffloadGet3(1).isUniqueTight
        require(stages.offloadSet4.stallVertex.length == 2)
        stages.offloadSet4.stallVertex(0) := offloadDefectMatch.io.condition
        stages.offloadSet4.stallVertex(1) := offloadDefectMatch.io.condition
        return ()
      case None =>
    }
    offloader.vm match {
      case Some(virtualMatch) =>
        val virtualVertex = virtualMatch.v.toInt
        val edgeIndex = virtualMatch.e.toInt
        val regularVertex = config.peerVertexOfEdge(edgeIndex, virtualVertex)
        var offloadVirtualMatch = OffloadVirtualMatch(io.vertexInputsOffloadGet3.length - 2)
        offloadVirtualMatch.io.edgeIsTight := io.edgeInputOffloadGet3.isTight
        for ((vertexIndex, localIndex) <- config.offloaderNeighborVertexIndices(offloaderIndex).zipWithIndex) {
          val vertexOffloadGet3 = io.vertexInputsOffloadGet3(localIndex)
          if (vertexIndex == virtualVertex) {
            offloadVirtualMatch.io.virtualIsVirtual := vertexOffloadGet3.state.isVirtual
            stages.offloadSet4.stallVertex(localIndex) := offloadVirtualMatch.io.condition
          } else if (vertexIndex == regularVertex) {
            offloadVirtualMatch.io.regularIsDefect := vertexOffloadGet3.state.isDefect
            offloadVirtualMatch.io.regularSpeed := vertexOffloadGet3.state.speed
            stages.offloadSet4.stallVertex(localIndex) := offloadVirtualMatch.io.condition
          } else {
            val neighborEdgeOffloadGet3 = io.neighborEdgeInputsOffloadGet3(localIndex)
            offloadVirtualMatch.io.neighborEdgeIsTight(localIndex) := neighborEdgeOffloadGet3.isTight
            offloadVirtualMatch.io.neighborVertexIsUniqueTight(localIndex) := vertexOffloadGet3.isUniqueTight
            offloadVirtualMatch.io.neighborVertexIsDefect(localIndex) := vertexOffloadGet3.state.isDefect
            stages.offloadSet4.stallVertex(localIndex) := offloadVirtualMatch.io.neighborVertexStalled(localIndex)
          }
        }
        return ()
      case None =>
    }
    throw new Exception("unrecognized definition of offloader")
  }
  connectLogic()

  // inject registers
  for (stageName <- injectRegisters) {
    stages.injectRegisterAt(stageName)
  }
  stages.finish()

}

// sbt 'testOnly microblossom.modules.OffloaderTest'
class OffloaderTest extends AnyFunSuite {

  test("construct an Offloader") {
    // val (config, offloaderIndex) = (DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json"), 0)
    val (config, offloaderIndex) = (DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json"), 531)
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Offloader(config, offloaderIndex))
  }

}

// sbt 'testOnly microblossom.modules.OffloaderEstimation'
class OffloaderEstimation extends AnyFunSuite {

  test("logic delay") {
    def dualConfig(name: String): DualConfig = {
      DualConfig(filename = s"./resources/graphs/example_$name.json"),
    }
    val configurations = List(
      // 2xLUT5
      (dualConfig("code_capacity_d5"), 0, "code capacity, defect match (3 / 5)"),
      // 2xLUT5, 1xLUT4 -> 3
      (dualConfig("code_capacity_d5"), 3, "code capacity, virtual match (2 / 5)"),
      // 2xLUT5
      (dualConfig("code_capacity_rotated_d5"), 0, "code capacity rotated, defect match (15 / 25)"),
      // 2xLUT6, 3xLUT5, 1xLUT4 -> 6
      (dualConfig("code_capacity_rotated_d5"), 15 + 5, "code capacity rotated, virtual match (10 / 25)"),
      // 2xLUT5
      (dualConfig("phenomenological_rotated_d5"), 0, "phenomenological, defect match (150 / 270)"),
      // 2xLUT6, 3xLUT5, 1xLUT4 -> 6
      (dualConfig("phenomenological_rotated_d5"), 150 + 60, "phenomenological, virtual match (120 / 270)"),
      // 2xLUT5
      (dualConfig("circuit_level_d5"), 0, "circuit-level, defect match (501, 561)"),
      // 12xLUT6, 1xLUT2 -> 13
      (dualConfig("circuit_level_d5"), 501 + 30, "circuit-level, virtual match (60, 561)")
    )
    for ((config, offloaderIndex, name) <- configurations) {
      val reports = Vivado.report(Offloader(config, offloaderIndex))
      println(s"$name:")
      reports.resource.primitivesTable.print()
    }
  }

}
