package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite
import microblossom.types._

object EdgeRemaining {
  def build(
      remaining: UInt, // output
      leftGrown: UInt,
      rightGrown: UInt,
      weight: UInt
  ) = {
    val weightWidth = weight.getWidth
    require(leftGrown.getWidth >= weightWidth)
    require(rightGrown.getWidth >= weightWidth)
    require(remaining.getWidth == weightWidth)

    val leftGrownTruncated = leftGrown.resize(weightWidth).resize(weightWidth + 1)
    val rightGrownTruncated = rightGrown.resize(weightWidth).resize(weightWidth + 1)
    val sum = UInt((weightWidth + 1) bits)
    sum := leftGrownTruncated + rightGrownTruncated

    def overflowed(grown: UInt): Bool = {
      if (grown.getWidth > weightWidth) {
        grown(grown.high downto weightWidth).orR
      } else {
        False
      }
    }
    val isOverflowed = overflowed(leftGrown) || overflowed(rightGrown)

    remaining := 0
    when(!isOverflowed && sum <= weight) {
      remaining := weight - sum.resized
    }
  }
}

case class EdgeRemaining(leftGrownBits: Int, rightGrownBits: Int, weightBits: Int) extends Component {
  require(leftGrownBits >= weightBits)
  require(rightGrownBits >= weightBits)

  val io = new Bundle {
    val leftGrown = in(UInt(leftGrownBits bits))
    val rightGrown = in(UInt(rightGrownBits bits))
    val weight = in(UInt(weightBits bits))

    val remaining = out(UInt(weightBits bits))
  }

  EdgeRemaining.build(io.remaining, io.leftGrown, io.rightGrown, io.weight)

}

// sbt 'testOnly microblossom.combinatorial.EdgeRemainingTest'
class EdgeRemainingTest extends AnyFunSuite {

  test("example") {
    val grownBits = 6
    val weightBits = 3
    Config.spinal().generateVerilog(EdgeRemaining(grownBits, grownBits, weightBits))
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
        .compile(EdgeRemaining(grownBits, grownBits, weightBits))
        .doSim("logic validity") { dut =>
          for (weight <- Range(0, 1 << weightBits)) {
            for (leftGrown <- Range(0, 1 << grownBits)) {
              for (rightGrown <- Range(0, 1 << grownBits)) {
                dut.io.weight #= weight
                dut.io.leftGrown #= leftGrown
                dut.io.rightGrown #= rightGrown
                sleep(1)
                val groundTruth = (weight - (leftGrown + rightGrown)).max(0)
                // println(weight, leftGrown, rightGrown, groundTruth)
                assert(dut.io.remaining.toInt == groundTruth, s"($leftGrown, $rightGrown, $weight)")
              }
            }
          }
        }
    }
  }

}
