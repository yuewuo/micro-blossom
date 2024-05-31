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

// one can attach a tag to the input: it will be returned alongside the message
case class MicroBlossomLooper[T <: Data](config: DualConfig, tagType: HardType[T] = EmptyTag()) extends Component {
  require(
    config.readLatency > 0,
    "consider adding broadcast delay; looper does not accept pure combinatorial implementation"
  )

  val io = new Bundle {
    val push = slave Stream (LooperInput(config, tagType))
    val pop = master Stream (LooperOutput(config, tagType))
    val dataLoss = out(Bool())
  }

  // define variables
  val immediateLoopback = Bool()
  val isDataRace = Bool()
  val inputInstruction = Instruction(config)
  val inputEntry = PipelineEntry(config, tagType)
  val pipelineLength = config.readLatency
  val pipelineEntries = Vec.fill(pipelineLength)(Reg(PipelineEntry(config, tagType)).initDefault())
  val responseEntry = pipelineEntries(pipelineLength - 1)
  val dataLoss = Reg(Bool()) init False
  val growLength = UInt(16 bits)

  io.dataLoss := dataLoss

  // create MicroBlossom module
  val microBlossom = DistributedDual(config, config)

  // immediate feedback happens when the response allows immediate growth
  // when maximumGrowth is 0, the loopback is forbidden
  immediateLoopback := responseEntry.valid && (
    responseEntry.isLoopBackGrow || (
      !microBlossom.io.conflict.valid &&
        (microBlossom.io.maxGrowable.length =/= microBlossom.io.maxGrowable.length.maxValue) &&
        (responseEntry.grown < responseEntry.maximumGrowth)
    )
  )

  // the input entry to the MicroBlossom module
  growLength := Mux( // the growth value of issuing another grow instruction in the loop back
    responseEntry.grown + microBlossom.io.maxGrowable.length.resize(16) > responseEntry.maximumGrowth,
    responseEntry.maximumGrowth - responseEntry.grown,
    microBlossom.io.maxGrowable.length.resize(16)
  )
  when(immediateLoopback) {
    inputEntry.valid := True
    if (config.contextBits > 0) { inputEntry.contextId := responseEntry.contextId }
    if (tagType != null) inputEntry.tag := responseEntry.tag
    inputEntry.maximumGrowth := responseEntry.maximumGrowth
    when(responseEntry.isLoopBackGrow) {
      inputEntry.grown := responseEntry.grown
      inputEntry.isLoopBackGrow := False
      inputInstruction.assignFindObstacle()
    } otherwise {
      inputEntry.grown := responseEntry.grown + growLength
      inputEntry.isLoopBackGrow := True
      inputInstruction.assignGrow(growLength)
    }
  } otherwise {
    inputEntry.valid := io.push.valid && !isDataRace
    if (config.contextBits > 0) { inputEntry.contextId := io.push.payload.contextId }
    if (tagType != null) inputEntry.tag := io.push.payload.tag
    inputEntry.maximumGrowth := io.push.payload.maximumGrowth
    inputEntry.grown := 0
    inputEntry.isLoopBackGrow := False
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
  val dataRaces = Vec.fill(pipelineLength)(Bool())
  for (i <- (0 until pipelineLength)) {
    val entry = pipelineEntries(i)
    if (config.contextBits > 0) {
      dataRaces(i) := entry.valid && entry.contextId === io.push.payload.contextId
    } else {
      dataRaces(i) := entry.valid
    }
  }
  isDataRace := dataRaces.reduceBalancedTree(_ | _)

  // output safety: the host bus should be much faster; if congestion detected, must reset the whole module
  when(responseEntry.valid && !immediateLoopback) {
    when(!io.pop.ready) {
      dataLoss := True
    }
    io.pop.valid := True
    if (config.contextBits > 0) { io.pop.payload.contextId := responseEntry.contextId }
    if (tagType != null) io.pop.payload.tag := responseEntry.tag
    io.pop.payload.maxGrowable := microBlossom.io.maxGrowable
    io.pop.payload.conflict.assignReordered(microBlossom.io.conflict)
    io.pop.payload.parityReports := microBlossom.io.parityReports
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
    if (tagType != null) io.push.payload.tag.assignDontCare()
    if (config.contextBits > 0) {
      io.push.payload.contextId #= input.contextId
    }
    io.pop.ready #= true
    io.push.payload.maximumGrowth #= input.maximumGrowth
    clockDomain.waitSamplingWhere(io.push.ready.toBoolean)
    io.push.valid #= false
    clockDomain.waitSamplingWhere(io.pop.valid.toBoolean)
    io.pop.ready #= false
    val valid = io.pop.payload.conflict.valid.toBoolean
    val node1 = io.pop.payload.conflict.node1.toInt
    val node2 = io.pop.payload.conflict.node2.toInt
    val touch1 = io.pop.payload.conflict.touch1.toInt
    val touch2 = io.pop.payload.conflict.touch2.toInt
    val vertex1 = io.pop.payload.conflict.vertex1.toInt
    val vertex2 = io.pop.payload.conflict.vertex2.toInt
    if (valid) {
      assert(node1 != config.IndexNone)
      assert(touch1 != config.IndexNone)
      assert(vertex1 != config.IndexNone)
      assert(vertex2 != config.IndexNone)
    }
    val option_node2: Option[Int] = if (node2 == config.IndexNone) { None }
    else { Some(node2) }
    val option_touch2: Option[Int] = if (touch2 == config.IndexNone) { None }
    else { Some(touch2) }
    LooperOutputData(
      contextId = if (config.contextBits > 0) {
        io.pop.payload.contextId.toInt
      } else {
        0
      },
      maxGrowable = io.pop.payload.maxGrowable.length.toInt,
      conflict = DataConflict(valid, node1, option_node2, touch1, option_touch2, vertex1, vertex2),
      grown = io.pop.payload.grown.toInt
    )
  }

  def simMakePublicSnapshot() = microBlossom.simMakePublicSnapshot()
  def simSnapshot(abbrev: Boolean = true): Json = microBlossom.simSnapshot(abbrev)
  def simMakePublicPreMatching() = microBlossom.simMakePublicPreMatching()
  def simPreMatchings(): Seq[DataPreMatching] = microBlossom.simPreMatchings()

}

case class EmptyTag() extends Bundle {}

case class LooperInput[T <: Data](config: DualConfig, tagType: HardType[T] = EmptyTag()) extends Bundle {
  val instruction = Instruction(config)
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val maximumGrowth = UInt(16 bits)
  val tag = (tagType != null) generate cloneOf(tagType)
}

case class LooperOutput[T <: Data](config: DualConfig, tagType: HardType[T] = EmptyTag()) extends Bundle {
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val maxGrowable = ConvergecastMaxGrowable(config.weightBits)
  val conflict = ConvergecastConflict(config.vertexBits)
  val grown = UInt(16 bits)
  val parityReports = Bits(config.parityReportersNum bits)
  val tag = (tagType != null) generate cloneOf(tagType)
}

case class PipelineEntry[T <: Data](config: DualConfig, tagType: HardType[T] = EmptyTag()) extends Bundle {
  val valid = Bool
  val contextId = (config.contextBits > 0) generate UInt(config.contextBits bits)
  val maximumGrowth = UInt(16 bits)
  val grown = UInt(16 bits)
  // bug 2024.5.25: the conflict reported after a loopback Grow instruction is not trustworthy:
  // the offloading module has not taken effect. The fix is to always issue a FindObstacle
  // instruction after the loopback Grow instruction (isLoopBackGrow := True)
  val isLoopBackGrow = Bool
  val tag = (tagType != null) generate cloneOf(tagType)

  def initDefault(): PipelineEntry[T] = {
    val defaultEntry = PipelineEntry(config, tagType)
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
    var maximumGrowth: Int
)

object LooperInputData {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

@ConfiguredJsonCodec
case class LooperOutputData(
    var contextId: Int,
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
