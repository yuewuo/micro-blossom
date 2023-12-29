package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite
import microblossom.types._

object VertexPostExecuteStateCommon {
  def build(
      after: VertexState, // output
      before: VertexState,
      message: BroadcastMessage,
      config: DualConfig,
      isVertexEqField1: Bool // should be `instruction.field1 === vertexIndex`, when config.supportAddDefectVertex is true
  ) = {

    val instruction = message.instruction

    after := before
    when(message.valid) {
      when(instruction.isSetSpeed) {
        when(before.node === instruction.field1) {
          after.speed := instruction.speed
        }
      }
      when(instruction.isSetBlossom) {
        when(before.node === instruction.field1 || before.root === instruction.field1) {
          after.node := instruction.field2
          after.speed := Speed.Grow
        }
      }
      when(instruction.isGrow) {
        switch(before.speed.asUInt) {
          is(Speed.Grow) {
            after.grown := before.grown + instruction.length.resized
          }
          is(Speed.Shrink) {
            after.grown := before.grown - instruction.length.resized
          }
        }
      }
      if (config.supportAddDefectVertex) {
        when(instruction.isAddDefect) {
          when(isVertexEqField1) {
            after.isDefect := True
            after.speed := Speed.Grow
            assert(
              assertion = before.node === config.IndexNone,
              message = "Cannot set a vertex to defect when it's already occupied",
              severity = ERROR
            )
            after.node := instruction.extendedField2.resized
            after.root := instruction.extendedField2.resized
          }
        }
      }
    }

  }
}

case class VertexPostExecuteStateCommon(config: DualConfig, grownBits: Int) extends Component {
  val io = new Bundle {
    val before = in(VertexState(config.vertexBits, grownBits))
    val message = in(BroadcastMessage(config))
    val isVertexEqField1 = in(Bool)

    val after = out(VertexState(config.vertexBits, grownBits))
  }

  VertexPostExecuteStateCommon.build(io.after, io.before, io.message, config, io.isVertexEqField1)
}

case class VertexPostExecuteState(config: DualConfig, vertexIndex: Int) extends Component {
  val grownBits = config.grownBitsOf(vertexIndex)

  val io = new Bundle {
    val before = in(VertexState(config.vertexBits, grownBits))
    val message = in(BroadcastMessage(config))

    val after = out(VertexState(config.vertexBits, grownBits))
  }

  val common = VertexPostExecuteStateCommon(config, grownBits)
  common.io.before := io.before
  common.io.message := io.message
  common.io.isVertexEqField1 := (io.message.instruction.field1 === vertexIndex)
  io.after := common.io.after

}

// sbt 'testOnly microblossom.combinatorial.VertexPostExecuteStateTest'
class VertexPostExecuteStateTest extends AnyFunSuite {

  test("example") {
    val config = DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json")
    val vertexIndex = 3
    Config.spinal().generateVerilog(VertexPostExecuteState(config, vertexIndex))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexPostExecuteStateDelayEstimation'
class VertexPostExecuteStateDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_d5.json"),
        1,
        "code capacity 2 neighbors"
      ), // 0.85ns / 0.99ns (LUT4 -> LUT6 -> LUT6)
      (
        DualConfig(filename = "./resources/graphs/example_code_capacity_rotated_d5.json"),
        10,
        "code capacity 4 neighbors"
      ), // 0.85ns / 0.99ns (LUT6 -> LUT6 -> LUT6)
      (
        DualConfig(filename = "./resources/graphs/example_phenomenological_rotated_d5.json"),
        64,
        "phenomenological 6 neighbors"
      ), // 1.00ns / 1.14ns (LUT4 -> LUT4 -> LUT5 -> LUT6)
      (
        DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json"),
        63,
        "circuit-level 12 neighbors"
      ) // 1.10ns / 1.10ns (LUT6 -> CARRY4 -> LUT4 -> LUT6) vertex: 9 bits, grown: 3 bits
    )
    for ((config, vertexIndex, name) <- configurations) {
      for (supportAddDefectVertex <- List(false, true)) {
        config.supportAddDefectVertex = supportAddDefectVertex
        val timingReport = Vivado.reportTiming(VertexPostExecuteState(config, vertexIndex))
        println(s"$name ($supportAddDefectVertex): ${timingReport.getPathDelaysExcludingIOWorst}ns")
      }
    }
  }

}
