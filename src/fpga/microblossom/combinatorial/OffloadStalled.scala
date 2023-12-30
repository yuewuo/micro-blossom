package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

object OffloadStalled {

  def build(
      isStalled: Bool, // output
      conditions: Seq[Bool]
  ) = {

    if (conditions.length > 0) {
      isStalled := conditions.orR
    } else {
      isStalled := False
    }

  }
}

case class OffloadStalled(numConditions: Int) extends Component {

  val io = new Bundle {
    val conditions = in(Vec.fill(numConditions)(Bool))

    val isStalled = out(Bool)
  }

  OffloadStalled.build(
    io.isStalled,
    io.conditions
  )

}

// sbt 'testOnly microblossom.combinatorial.OffloadStalledTest'
class OffloadStalledTest extends AnyFunSuite {

  test("example") {
    val numConditions = 12
    Config.spinal().generateVerilog(OffloadStalled(numConditions))
  }

}

// sbt 'testOnly microblossom.combinatorial.OffloadStalledEstimation'
class OffloadStalledEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      // delay: 0.04ns
      // resource: 1xLUT2
      (2, "code capacity 2 neighbors"),
      // delay: 0.04ns
      // resource: 1xLUT4
      (4, "code capacity 4 neighbors"),
      // delay: 0.04ns
      // resource: 1xLUT6
      (6, "phenomenological 6 neighbors"),
      // delay: 0.36ns
      // resource: 2xLUT6, 1xLUT2
      (12, "circuit-level 12 neighbors")
    )
    for ((numConditions, name) <- configurations) {
      val reports = Vivado.report(OffloadStalled(numConditions))
      println(s"$name: ${reports.timing.getPathDelaysExcludingIOWorst}ns")
      reports.resource.primitivesTable.print()
    }
  }

}
