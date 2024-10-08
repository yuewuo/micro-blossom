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
      isVertexEqField1: Bool, // should be `instruction.field1 === vertexIndex`, when config.supportAddDefectVertex is true
      isLayerIdEqField1: Bool,
      isStalled: Bool
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
        when(!isStalled) {
          switch(before.speed.asUInt) {
            is(Speed.Grow) {
              after.grown := before.grown + instruction.length.resized
            }
            is(Speed.Shrink) {
              after.grown := before.grown - instruction.length.resized
            }
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
            after.node := instruction.field2.resized
            after.root := instruction.field2.resized
          }
        }
      }
      if (config.supportLayerFusion) {
        when(instruction.isLoadDefectsExternal && isLayerIdEqField1) {
          after.isVirtual := False
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
    val isLayerIdEqField1 = in(Bool)
    val isStalled = in(Bool)

    val after = out(VertexState(config.vertexBits, grownBits))
  }

  VertexPostExecuteStateCommon.build(
    io.after,
    io.before,
    io.message,
    config,
    io.isVertexEqField1,
    io.isLayerIdEqField1,
    io.isStalled
  )
}

case class VertexPostExecuteState(config: DualConfig, vertexIndex: Int) extends Component {
  val grownBits = config.grownBitsOf(vertexIndex)

  val io = new Bundle {
    val before = in(VertexState(config.vertexBits, grownBits))
    val message = in(BroadcastMessage(config))
    val isStalled = in(Bool)

    val after = out(VertexState(config.vertexBits, grownBits))
  }

  val common = VertexPostExecuteStateCommon(config, grownBits)
  common.io.before := io.before
  common.io.message := io.message
  common.io.isVertexEqField1 := (io.message.instruction.field1 === vertexIndex)
  if (config.vertexLayerId.contains(vertexIndex)) {
    val layerId = config.vertexLayerId(vertexIndex)
    common.io.isLayerIdEqField1 := (io.message.instruction.field1 === layerId)
    common.io.isStalled := io.isStalled || io.before.isVirtual
  } else {
    common.io.isLayerIdEqField1 := False
    common.io.isStalled := io.isStalled
  }
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

// sbt 'runMain microblossom.combinatorial.VertexPostExecuteStateEstimation'
object VertexPostExecuteStateEstimation extends App {
  def dualConfig(name: String): DualConfig = {
    DualConfig(filename = s"./resources/graphs/example_$name.json"),
  }
  val configurations = List(
    // delay: 0.85ns / 0.99ns (LUT4 -> LUT6 -> LUT6)
    // resource: (5xLUT6, 5xLUT5, 9xLUT4, 1xLUT3 -> 20) / (10xLUT6, 8xLUT5, 5xLUT4, 2xLUT3, 5xLUT2 -> 30)
    (dualConfig("code_capacity_d5"), 1, "code capacity 2 neighbors"),
    // delay: 0.85ns / 0.99ns (LUT6 -> LUT6 -> LUT6)
    // resource: (7xLUT6, 5xLUT5, 8xLUT4, 1xLUT3 -> 21) / (13xLUT6, 7xLUT5, 4xLUT4, 2xLUT3, 6xLUT2 -> 32)
    (dualConfig("code_capacity_rotated_d5"), 10, "code capacity 4 neighbors"),
    // delay: 1.00ns / 1.14ns (LUT4 -> LUT4 -> LUT5 -> LUT6)
    // resource: (22xLUT6, 1xLUT5, 2xLUT4, 1xLUT3, 3xLUT2 -> 29) / (11xLUT6, 7xLUT5, 16xLUT4, 6xLUT3 -> 40)
    (dualConfig("phenomenological_rotated_d5"), 64, "phenomenological 6 neighbors"),
    // delay:  1.10ns / 1.10ns (LUT6 -> CARRY4 -> LUT4 -> LUT6) vertex: 9 bits, grown: 3 bits
    // resource: (10xLUT6, 5xLUT5, 12xLUT4, 2xLUT3, 2xCARRAY4 -> 31) / (19xLUT6, 8xLUT5, 8xLUT4, 7xLUT3, 2xCARRAY4 -> 44)
    (dualConfig("circuit_level_d5"), 63, "circuit-level 12 neighbors")
  )
  for ((config, vertexIndex, name) <- configurations) {
    for (supportAddDefectVertex <- List(false, true)) {
      config.supportAddDefectVertex = supportAddDefectVertex
      val reports = Vivado.report(VertexPostExecuteState(config, vertexIndex))
      println(s"$name ($supportAddDefectVertex): ${reports.timing.getPathDelaysExcludingIOWorst}ns")
      reports.resource.primitivesTable.print()
    }
  }
}
