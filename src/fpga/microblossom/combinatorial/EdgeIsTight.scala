package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite

object EdgeIsTight {
  def build(
      isTight: Bool, // output
      leftGrown: UInt,
      rightGrown: UInt,
      weight: UInt
  ) = {
    val weightWidth = weight.getWidth
    require(leftGrown.getWidth >= weightWidth)
    require(rightGrown.getWidth >= weightWidth)
    // usually the grown bits are much larger than the weight, e.g. weight is 3 bits but grown is 7 bits
    // we could optimize the logic so that it uses fewer resources
    val leftGrownTruncated = leftGrown.resize(weightWidth).resize(weightWidth + 1)
    val rightGrownTruncated = rightGrown.resize(weightWidth).resize(weightWidth + 1)
    def overflowed(grown: UInt): Bool = {
      if (grown.getWidth > weightWidth) {
        grown(grown.high downto weightWidth).orR
      } else {
        False
      }
    }
    val isOverflowed = overflowed(leftGrown) || overflowed(rightGrown)
    isTight := ((leftGrownTruncated + rightGrownTruncated) >= weight) || isOverflowed
  }
}

case class EdgeIsTight(leftGrownBits: Int, rightGrownBits: Int, weightBits: Int) extends Component {
  require(leftGrownBits >= weightBits)
  require(rightGrownBits >= weightBits)

  val io = new Bundle {
    val leftGrown = in(UInt(leftGrownBits bits))
    val rightGrown = in(UInt(rightGrownBits bits))
    val weight = in(UInt(weightBits bits))
    val isTight = out(Bool)
  }

  EdgeIsTight.build(io.isTight, io.leftGrown, io.rightGrown, io.weight)

}

// sbt 'testOnly microblossom.combinatorial.EdgeIsTightTest'
class EdgeIsTightTest extends AnyFunSuite {

  test("example") {
    val grownBits = 6
    val weightBits = 3
    Config.spinal().generateVerilog(EdgeIsTight(grownBits, grownBits, weightBits))
  }

  test("logic validity") {
    val configurations = List(
      (2, 2),
      (3, 2),
      (4, 4),
      (5, 4),
      (7, 4)
    )
    for ((grownBits, weightBits) <- configurations) {
      Config.sim
        .compile(EdgeIsTight(grownBits, grownBits, weightBits))
        .doSim("logic validity") { dut =>
          for (weight <- Range(0, 1 << weightBits)) {
            for (leftGrown <- Range(0, 1 << grownBits)) {
              for (rightGrown <- Range(0, 1 << grownBits)) {
                dut.io.weight #= weight
                dut.io.leftGrown #= leftGrown
                dut.io.rightGrown #= rightGrown
                sleep(1)
                val groundTruth = (leftGrown + rightGrown) >= weight
                assert(dut.io.isTight.toBoolean == groundTruth, s"($leftGrown, $rightGrown, $weight)")
              }
            }
          }
        }
    }
  }

}

// sbt 'testOnly microblossom.combinatorial.EdgeIsTightEstimation'
class EdgeIsTightEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      // delay: 0.04ns
      // resource: 1xLUT6
      (2, 2, "minimal for d=3 code"),
      // delay: 0.36ns
      // resource: 1xLUT6, 1xLUT3
      (3, 2, "minimal for d=5,7 code"),
      // delay: 0.36ns
      // resource: 1xLUT6, 1xLUT5
      (4, 2, "minimal for d=9,11,13,15 code"),
      // delay: 0.36ns
      // resource: 2xLUT6, 1xLUT2
      (5, 2, "minimal for d=[17, 31] code"),
      // delay: 0.67ns
      // resource: 2xLUT6, 2xLUT5, 1xLUT1
      (4, 4, "circuit-level for d=3 code"), // max_half_weight = 7
      // delay: 0.68ns
      // resource: 5xLUT6
      (5, 4, "circuit-level for d=5,7 code"),
      // delay: 0.67ns
      // resource: 3xLUT6, 1xLUT5, 2xLUT4
      (6, 4, "circuit-level for d=9,11,13,15 code"),
      // delay: 0.67ns
      // resource: 4xLUT6, 1xLUT5, 1xLUT4
      (7, 4, "circuit-level for d=[17, 31] code")
    )
    for ((grownBits, weightBits, name) <- configurations) {
      val reports = Vivado.report(EdgeIsTight(grownBits, grownBits, weightBits))
      println(s"$name: ${reports.timing.getPathDelaysExcludingIOWorst}ns")
      reports.resource.primitivesTable.print()
    }
  }

}
