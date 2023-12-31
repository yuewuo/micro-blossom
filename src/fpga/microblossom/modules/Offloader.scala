package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
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
    val edgeInputOffloadGet3 = in(Edge.getStages(config).getStageOutput.offloadGet3)
  }

  val stages = Offloader.getStages(config, offloaderIndex)
  stages.connectStageOutput(io.stageOutputs)

  stages.offloadSet4.stallVertex := Vec.fill(config.numOffloaderNeighborOf(offloaderIndex))(False) // TODO

  // inject registers
  for (stageName <- injectRegisters) {
    stages.injectRegisterAt(stageName)
  }
  stages.finish()

}

// sbt 'testOnly microblossom.modules.OffloaderTest'
class OffloaderTest extends AnyFunSuite {

  test("construct a Offloader") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Offloader(config, 0))
  }

}
