package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

object OffloadDefectMatch {
  def build(
      condition: Bool,
      edgeIsTight: Bool,
      leftIsDefect: Bool,
      leftSpeed: Speed,
      leftIsUniqueTight: Bool,
      rightIsDefect: Bool,
      rightSpeed: Speed,
      rightIsUniqueTight: Bool
  ) = {

    condition := edgeIsTight && leftIsDefect && (leftSpeed === Speed.Grow) && leftIsUniqueTight &&
      rightIsDefect && (rightSpeed === Speed.Grow) && rightIsUniqueTight

  }
}

case class OffloadDefectMatch() extends Component {

  val io = new Bundle {
    val edgeIsTight = in(Bool)

    val leftIsDefect = in(Bool)
    val leftSpeed = in(Speed())
    val leftIsUniqueTight = in(Bool)

    val rightIsDefect = in(Bool)
    val rightSpeed = in(Speed())
    val rightIsUniqueTight = in(Bool)

    val condition = out(Bool)
  }

  OffloadDefectMatch.build(
    io.condition,
    io.edgeIsTight,
    io.leftIsDefect,
    io.leftSpeed,
    io.leftIsUniqueTight,
    io.rightIsDefect,
    io.rightSpeed,
    io.rightIsUniqueTight
  )

}

// sbt 'testOnly microblossom.combinatorial.OffloadDefectMatchTest'
class OffloadDefectMatchTest extends AnyFunSuite {

  test("example") {
    Config.spinal().generateVerilog(OffloadDefectMatch())
  }

}

// sbt 'testOnly microblossom.combinatorial.OffloadDefectMatchEstimation'
class OffloadDefectMatchEstimation extends AnyFunSuite {

  test("logic delay") {
    // delay: 0.36ns (LUT5 -> LUT5)
    // resource: 2xLUT5
    val reports = Vivado.report(OffloadDefectMatch())
    println(s"${reports.timing.getPathDelaysExcludingIOWorst}ns")
    reports.resource.primitivesTable.print()
  }

}
