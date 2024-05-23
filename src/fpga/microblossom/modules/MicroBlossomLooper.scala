package microblossom.modules

/*
 * # Micro Blossom Looper
 *
 * Keeping sending `Grow` instruction until conflicts is detected or growable is infinity.
 * This module will also detect data races within the same context id and block the input stream if data race is detected.
 *
 */

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite
import scala.util.control.Breaks._
import scala.collection.mutable.ArrayBuffer
import scala.collection.mutable.Map

case class MicroBlossomLooper(config: DualConfig) extends Component {
  val io = new Bundle {
    // val input = in(Stream(InputData(ioConfig)))

    // val maxGrowable = out(ConvergecastMaxGrowable(ioConfig.weightBits))
    // val conflict = out(ConvergecastConflict(ioConfig.vertexBits))
  }

  // val pipelineEntries = Vec.fill(config.readLatency)(Reg(PipelineEntry(config)))
  // TODO: combinatorial logic to match context id with the one in the pipeline

}

case class InputData(config: DualConfig) extends Bundle {
  val instruction = Instruction(config)
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val instructionId = UInt(config.instructionBufferBits bits)
}

case class PipelineEntry(config: DualConfig) extends Bundle {
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val instructionId = UInt(config.instructionBufferBits bits)
}

// sbt 'testOnly *MicroBlossomLooperTest'
class MicroBlossomLooperTest extends AnyFunSuite {

  test("logic_validity") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    val clockDivideBy = 2

    Config.sim
      .compile(MicroBlossomLooper(config))
      .doSim("logic_validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

      }

  }

}
