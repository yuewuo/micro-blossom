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

  val isVertexEqField1 = (io.message.instruction.field1 === vertexIndex)

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
      (DualConfig(filename = "./resources/graphs/example_circuit_level_d5.json"), , "code capacity 2 neighbors"), // 0.04ns
      (4, "code capacity 4 neighbors"), // 0.04ns
      (6, "phenomenological 6 neighbors"), // 0.04ns
      (12, "circuit-level 12 neighbors") // 0.67ns (LUT6 -> LUT6 -> LUT4)
    )
    for ((numEdges, name) <- configurations) {
      val timingReport = Vivado.reportTiming(VertexPostExecuteState(numEdges))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
