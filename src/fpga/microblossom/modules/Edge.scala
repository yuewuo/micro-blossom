package microblossom.modules

import spinal.core._
import spinal.lib._
import microblossom._
import microblossom.types._
import microblossom.stage._
import org.scalatest.funsuite.AnyFunSuite

case class Edge(config: DualConfig, edgeIndex: Int, injectRegisters: Seq[String] = List()) extends Component {
  val io = new Bundle {
    val message = in(BroadcastMessage(config))
    val debugState = out(EdgeState(config.weightBits))
  }

  val stages = Stages(
    offload = () => StageOffloadEdge(config),
    offload2 = () => StageOffloadEdge2(config),
    offload3 = () => StageOffloadEdge3(config),
    offload4 = () => StageOffloadEdge4(config)
  )

  // fetch
  var ram: Mem[EdgeState] = null
  var register = Reg(EdgeState(config.weightBits))
  var fetchState = EdgeState(config.weightBits)
//   var message = BroadcastMessage(config)
  if (config.contextBits > 0) {
    // fetch stage, delay the instruction
    ram = Mem(EdgeState(config.weightBits), config.contextDepth)
    fetchState := ram.readSync(
      address = io.message.contextId,
      enable = io.message.valid
    )
    // message := RegNext(io.message)
  } else {
    fetchState := register
    // message := io.message
  }

  // mock
  stages.offloadSet.state := fetchState
  stages.offloadSet2.state := stages.offloadGet.state
  stages.offloadSet3.state := stages.offloadGet2.state
  stages.offloadSet4.state := stages.offloadGet3.state
  register := stages.offloadGet4.state

  // inject registers
  for (stageName <- injectRegisters) {
    stages.injectRegisterAt(stageName)
  }
  stages.finish()

  io.debugState := stages.offloadGet4.state

}

// sbt 'testOnly microblossom.modules.EdgeTest'
class EdgeTest extends AnyFunSuite {

  test("construct a Edge") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    // config.contextDepth = 1024 // fit in a single Block RAM of 36 kbits in 36-bit mode
    config.contextDepth = 1 // no context switch
    config.sanityCheck()
    Config.spinal().generateVerilog(Edge(config, 0))
  }

}
