package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite
import microblossom.types._

object EdgeResponse {
  def build(
      maxLength: ConvergecastMaxLength, // output
      conflict: ConvergecastConflict, // output
      leftShadow: VertexShadowResult,
      rightShadow: VertexShadowResult,
      leftVertex: Bits,
      rightVertex: Bits,
      remaining: UInt
  ) = {
    require(leftVertex.getWidth == conflict.vertex1.getWidth)
    require(rightVertex.getWidth == conflict.vertex2.getWidth)

    val isBothGrow = (leftShadow.speed === Speed.Grow && rightShadow.speed === Speed.Grow)
    val isJointSpeedPositive = isBothGrow ||
      (leftShadow.speed === Speed.Grow && rightShadow.speed === Speed.Stay) ||
      (leftShadow.speed === Speed.Stay && rightShadow.speed === Speed.Grow)

    maxLength.length := maxLength.length.maxValue
    conflict.valid := False
    conflict.node1 := leftShadow.node
    conflict.node2 := rightShadow.node
    conflict.touch1 := leftShadow.root
    conflict.touch2 := rightShadow.root
    conflict.vertex1 := leftVertex
    conflict.vertex2 := rightVertex
    when(leftShadow.node =/= rightShadow.node) {
      when(isJointSpeedPositive) {
        when(remaining === 0) {
          conflict.valid := True
        } otherwise {
          when(isBothGrow) {
            assert(
              assertion = remaining(0) === False,
              message = "when both ends are growing, the remaining length must be a even number",
              severity = ERROR
            )
            maxLength.length := remaining |>> 1
          } otherwise {
            maxLength.length := remaining
          }
        }
      }
    }

  }
}

case class EdgeResponse(vertexBits: Int, weightBits: Int) extends Component {

  val io = new Bundle {
    val leftShadow = in(VertexShadowResult(vertexBits))
    val rightShadow = in(VertexShadowResult(vertexBits))
    val leftVertex = in(Bits(vertexBits bits))
    val rightVertex = in(Bits(vertexBits bits))
    val remaining = in(UInt(weightBits bits))

    val maxLength = out(ConvergecastMaxLength(weightBits))
    val conflict = out(ConvergecastConflict(vertexBits))
  }

  EdgeResponse.build(
    io.maxLength,
    io.conflict,
    io.leftShadow,
    io.rightShadow,
    io.leftVertex,
    io.rightVertex,
    io.remaining
  )

}

// sbt 'testOnly microblossom.combinatorial.EdgeResponseTest'
class EdgeResponseTest extends AnyFunSuite {

  test("example") {
    val vertexBits = 8
    val weightBits = 3
    Config.spinal().generateVerilog(EdgeResponse(vertexBits, weightBits))
  }

}

// sbt 'testOnly microblossom.combinatorial.EdgeResponseDelayEstimation'
class EdgeResponseDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (3, 2, "minimal for d=3 code"), // 0.37ns
      (5, 2, "minimal for d=5,7 code"), // 0.37ns
      (7, 2, "minimal for d=9,11,13,15 code"), // 0.51ns
      (9, 2, "minimal for d=[17, 31] code"), // 0.74ns
      (4, 4, "circuit-level for d=3 code"), // max_half_weight = 7, 0.52ns
      (8, 4, "circuit-level for d=5,7 code"), // 0.65ns
      (11, 4, "circuit-level for d=9,11,13,15 code"), // 0.79ns
      (14, 4, "circuit-level for d=[17, 31] code") // 0.80ns
    )
    for ((vertexBits, weightBits, name) <- configurations) {
      val timingReport = Vivado.reportTiming(EdgeResponse(vertexBits, weightBits))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
