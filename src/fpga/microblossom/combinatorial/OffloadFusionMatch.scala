package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

object OffloadFusionMatch {
  def build(
      condition: Bool,
      edgeIsTight: Bool,
      conditionalIsVirtual: Bool,
      regularIsDefect: Bool,
      regularSpeed: Speed,
      regularIsIsolated: Bool
  ) = {

    condition := edgeIsTight && conditionalIsVirtual && regularIsDefect && (regularSpeed === Speed.Grow) && regularIsIsolated

  }
}

case class OffloadFusionMatch() extends Component {

  val io = new Bundle {
    val edgeIsTight = in(Bool)

    val conditionalIsVirtual = in(Bool)

    val regularIsDefect = in(Bool)
    val regularSpeed = in(Speed())
    val regularIsIsolated = in(Bool)

    val condition = out(Bool)
  }

  OffloadFusionMatch.build(
    io.condition,
    io.edgeIsTight,
    io.conditionalIsVirtual,
    io.regularIsDefect,
    io.regularSpeed,
    io.regularIsIsolated
  )

}

// sbt 'testOnly microblossom.combinatorial.OffloadFusionMatchTest'
class OffloadFusionMatchTest extends AnyFunSuite {

  test("example") {
    Config.spinal().generateVerilog(OffloadFusionMatch())
  }

}

// sbt 'runMain microblossom.combinatorial.OffloadFusionMatchEstimation'
object OffloadFusionMatchEstimation extends App {
  // TODO: delay: 0.36ns (LUT5 -> LUT5)
  // TODO: resource: 2xLUT5
  val reports = Vivado.report(OffloadFusionMatch())
  println(s"${reports.timing.getPathDelaysExcludingIOWorst}ns")
  reports.resource.primitivesTable.print()
}
