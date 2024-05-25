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
import org.scalatest.funsuite.AnyFunSuite
import scala.util.control.Breaks._
import scala.collection.mutable.ArrayBuffer
import scala.collection.mutable.Map

case class MicroBlossomLooper(config: DualConfig) extends Component {
  require(
    config.readLatency > 0,
    "consider adding broadcast delay; looper does not accept pure combinatorial implementation"
  )

  val io = new Bundle {
    val push = slave Stream (InputData(config))
    val pop = master Stream (OutputData(config))
    val dataLoss = out(Bool())
  }

  // define variables
  val immediateLoopback = Bool()
  val inputInstruction = Instruction(config)
  val inputEntry = PipelineEntry(config)
  val pipelineLength = config.readLatency
  val pipelineEntries = Vec.fill(pipelineLength)(Reg(PipelineEntry(config)).init_default())
  val responseEntry = pipelineEntries(pipelineLength - 1)
  val dataLoss = Reg(Bool()) init False
  io.dataLoss := dataLoss

  // create MicroBlossom module
  val microBlossom = DistributedDual(config, config)

  // immediate feedback happens when the response allows immediate growth
  immediateLoopback := !microBlossom.io.conflict.valid &&
    (microBlossom.io.maxGrowable.length =/= microBlossom.io.maxGrowable.length.maxValue)

  // the input entry to the MicroBlossom module
  when(immediateLoopback) {
    inputEntry.valid := True
    if (config.contextBits > 0) { inputEntry.contextId := responseEntry.contextId }
    inputEntry.instructionId := responseEntry.instructionId
    inputEntry.maximumGrowth := responseEntry.maximumGrowth
    val growLength = UInt(16 bits)
    when(responseEntry.grown + microBlossom.io.maxGrowable.length.resized > responseEntry.maximumGrowth) {
      growLength := responseEntry.maximumGrowth - responseEntry.grown
    } otherwise {
      growLength := microBlossom.io.maxGrowable.length.resized
    }
    inputEntry.grown := responseEntry.grown + growLength
    inputInstruction := inputInstruction.spec.dynamicGrow(growLength, config)
  } otherwise {
    inputEntry.valid := io.push.valid
    if (config.contextBits > 0) { inputEntry.contextId := io.push.payload.contextId }
    inputEntry.instructionId := io.push.payload.instructionId
    inputEntry.maximumGrowth := io.push.payload.maximumGrowth
    inputEntry.grown := 0
    inputInstruction := io.push.payload.instruction
  }

  // pass the inputEntry to the MicroBlossom module
  microBlossom.io.message.valid := inputEntry.valid
  microBlossom.io.message.instruction := inputInstruction
  if (config.contextBits > 0) { microBlossom.io.message.contextId := inputEntry.contextId }

  // shift pipeline entries
  for (i <- (0 until pipelineLength).reverse) {
    pipelineEntries(i) := (if (i == 0) { inputEntry }
                           else { pipelineEntries(i - 1) })
  }

  // detect data races: forbid an instruction to enter until the pipeline does not have any entry of the same context ID
  val isDataRace = if (config.contextBits > 0) {
    val dataRaces = Vec.fill(pipelineLength)(Bool())
    for (i <- (0 until pipelineLength)) {
      val entry = pipelineEntries(i)
      if (config.contextBits > 0) {
        dataRaces(i) := entry.valid && entry.contextId === io.push.payload.contextId
      } else {
        dataRaces(i) := entry.valid
      }
    }
    dataRaces.reduceBalancedTree(_ | _)
  } else { False }

  // output safety: the host bus should be much faster; if congestion detected, must reset the whole module
  when(responseEntry.valid && !immediateLoopback) {
    when(!io.pop.ready) {
      dataLoss := True
    }
    io.pop.valid := True
    if (config.contextBits > 0) { io.pop.payload.contextId := responseEntry.contextId }
    io.pop.payload.instructionId := responseEntry.instructionId
    io.pop.payload.maxGrowable := microBlossom.io.maxGrowable
    io.pop.payload.conflict := microBlossom.io.conflict
    io.pop.payload.grown := responseEntry.grown
  } otherwise {
    io.pop.valid := False
    io.pop.payload.assignDontCare()
  }

  // take the data from input only if it's valid, no data race, and not inserting immediate loopback
  io.push.ready := io.push.valid && !isDataRace && !immediateLoopback

  def simExecute(input: LooperInputData): LooperOutputData = {
    io.push.valid #= true
    io.push.payload.instruction #= input.instruction
    io.push.payload.instructionId #= input.instructionId
    if (config.contextBits > 0) {
      io.push.payload.contextId #= input.contextId
    }
    io.pop.ready #= true
    io.push.payload.maximumGrowth #= input.maximumGrowth
    clockDomain.waitSamplingWhere(io.push.ready.toBoolean)
    io.push.valid #= false
    clockDomain.waitSamplingWhere(io.pop.valid.toBoolean)
    io.pop.ready #= false
    LooperOutputData(
      contextId = if (config.contextBits > 0) {
        io.pop.payload.contextId.toInt
      } else {
        0
      },
      instructionId = io.pop.payload.instructionId.toInt,
      maxGrowable = io.pop.payload.maxGrowable.length.toInt,
      conflict = DataConflict(
        io.pop.payload.conflict.valid.toBoolean,
        io.pop.payload.conflict.node1.toInt,
        io.pop.payload.conflict.node2.toInt,
        io.pop.payload.conflict.touch1.toInt,
        io.pop.payload.conflict.touch2.toInt,
        io.pop.payload.conflict.vertex1.toInt,
        io.pop.payload.conflict.vertex2.toInt
      ),
      grown = io.pop.payload.grown.toInt
    )
  }
  def simMakePublicSnapshot() = {
    microBlossom.simMakePublicSnapshot()
  }
  def simSnapshot(abbrev: Boolean = true): Json = {
    microBlossom.simSnapshot(abbrev)
  }
}

case class InputData(config: DualConfig) extends Bundle {
  val instruction = Instruction(config)
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val instructionId = UInt(config.instructionBufferBits bits)
  val maximumGrowth = UInt(16 bits)
}

case class OutputData(config: DualConfig) extends Bundle {
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val instructionId = UInt(config.instructionBufferBits bits)
  val maxGrowable = ConvergecastMaxGrowable(config.weightBits)
  val conflict = ConvergecastConflict(config.vertexBits)
  val grown = UInt(16 bits)
}

case class PipelineEntry(config: DualConfig) extends Bundle {
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val instructionId = UInt(config.instructionBufferBits bits)
  val maximumGrowth = UInt(16 bits)
  val grown = UInt(16 bits)

  def init_default(): PipelineEntry = {
    val defaultEntry = PipelineEntry(config)
    defaultEntry.valid := False
    defaultEntry.assignDontCareToUnasigned()
    this.init(defaultEntry)
    this
  }
}

@ConfiguredJsonCodec
case class LooperInputData(
    var instruction: Long,
    var contextId: Int,
    var instructionId: Int,
    var maximumGrowth: Int
)

object LooperInputData {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

@ConfiguredJsonCodec
case class LooperOutputData(
    var contextId: Int,
    var instructionId: Int,
    var maxGrowable: Int,
    var conflict: DataConflict,
    var grown: Int
)

object LooperOutputData {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

// sbt 'testOnly *MicroBlossomLooperTest'
class MicroBlossomLooperTest extends AnyFunSuite {

  val filename = "./resources/graphs/example_code_capacity_d3.json"
  val logic_validity_config = Seq(
    DualConfig(filename = filename, broadcastDelay = 1),
    DualConfig(filename = filename, broadcastDelay = 2),
    DualConfig(filename = filename, broadcastDelay = 3),
    DualConfig(filename = filename, broadcastDelay = 1, contextDepth = 2),
    DualConfig(filename = filename, broadcastDelay = 1, contextDepth = 4),
    DualConfig(filename = filename, broadcastDelay = 1, contextDepth = 8),
    DualConfig(filename = filename, broadcastDelay = 3, contextDepth = 4)
  )

  test("logic_validity") {
    for ((config, i) <- logic_validity_config.zipWithIndex) {
      Config.sim
        .compile(MicroBlossomLooper(config))
        .doSim("logic_validity") { dut =>
          dut.clockDomain.forkStimulus(period = 10)
          dut.io.pop.ready #= true
          dut.io.push.valid #= false
          if (config.contextBits > 0) { dut.io.push.payload.contextId #= 0 }

          for (idx <- 0 to 5) { dut.clockDomain.waitSampling() }

          dut.io.push.valid #= true
          if (config.contextBits > 0) { dut.io.push.payload.contextId #= config.contextDepth - 1 }
          dut.clockDomain.waitSampling()
          dut.io.push.valid #= false
          if (config.contextBits > 0) { dut.io.push.payload.contextId #= 0 }

          for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        }

    }
  }

}
