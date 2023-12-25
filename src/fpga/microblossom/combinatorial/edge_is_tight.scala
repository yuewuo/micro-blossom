package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import org.scalatest.funsuite.AnyFunSuite

object EdgeIsTight {
  def build(isTight: Bool, leftGrown: UInt, rightGrown: UInt, weight: UInt) = {
    val weightWidth = weight.getWidth
    assert(leftGrown.getWidth >= weightWidth)
    assert(rightGrown.getWidth >= weightWidth)
    // usually the grown bits are much larger than the weight, e.g. weight is 3 bits but grown is 7 bits
    // we could optimize the logic so that it uses less resources
    // TODO: implement this optimization and make it optional given the config
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

// sbt 'testOnly microblossom.combinatorial.EdgeIsTightTest'
class EdgeIsTightTest extends AnyFunSuite {

  case class EdgeIsTightTester(grownBits: Int, weightBits: Int) extends Component {
    assert(grownBits >= weightBits)

    val io = new Bundle {
      val leftGrown = in(UInt(grownBits bits))
      val rightGrown = in(UInt(grownBits bits))
      val weight = in(UInt(weightBits bits))
      val isTight = out(Bool)
    }

    EdgeIsTight.build(io.isTight, io.leftGrown, io.rightGrown, io.weight)

  }

  test("example") {
    val weightBits = 3
    val grownBits = 6
    Config.spinal().generateVerilog(EdgeIsTightTester(grownBits, weightBits))
  }

  test("logic validity") {
    val weightBits = 3
    for (grownBits <- Range(3, 8)) {
      Config.sim
        .compile(EdgeIsTightTester(grownBits, weightBits))
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
