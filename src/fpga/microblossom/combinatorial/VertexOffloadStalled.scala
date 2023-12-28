package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

object VertexOffloadStalled {

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

case class VertexOffloadStalled(numConditions: Int) extends Component {

  val io = new Bundle {
    val conditions = in(Vec.fill(numConditions)(Bool))

    val isStalled = out(Bool)
  }

  VertexOffloadStalled.build(
    io.isStalled,
    io.conditions
  )

}

// sbt 'testOnly microblossom.combinatorial.VertexOffloadStalledTest'
class VertexOffloadStalledTest extends AnyFunSuite {

  test("example") {
    val numConditions = 12
    Config.spinal().generateVerilog(VertexOffloadStalled(numConditions))
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexOffloadStalledDelayEstimation'
class VertexOffloadStalledDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (2, "code capacity 2 neighbors"), // 0.04ns
      (4, "code capacity 4 neighbors"), // 0.04ns
      (6, "phenomenological 6 neighbors"), // 0.04ns
      (12, "circuit-level 12 neighbors") // 0.36ns
    )
    for ((numConditions, name) <- configurations) {
      val timingReport = Vivado.reportTiming(VertexOffloadStalled(numConditions))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
