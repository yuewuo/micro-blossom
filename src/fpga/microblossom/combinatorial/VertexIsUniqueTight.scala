package microblossom.combinatorial

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util.Vivado
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

object VertexIsUniqueTight {
  def build(isUnique: Bool, tights: Seq[Bool]) = {
    require(tights.length > 0)

    val num = tights.length

    val sliceHasTight = mutable.ArrayBuffer[Bool]() ++ tights
    val sliceIsUnique = mutable.ArrayBuffer[Bool]() ++ Vec.fill(num)(True)

    var startIndex = 0
    var endIndex = num
    while (endIndex > startIndex + 1) {
      val sliceNum = endIndex - startIndex
      val numGroups = 1 + ((sliceNum - 1) / 2)
      for (groupIndex <- 0 until numGroups) {
        val index = startIndex + groupIndex * 2
        val groupNum = ((groupIndex + 1) * 2).min(sliceNum) - groupIndex * 2
        assert(groupNum == 1 || groupNum == 2)
        if (groupNum == 1) {
          sliceHasTight.append(sliceHasTight(index))
          sliceIsUnique.append(sliceIsUnique(index))
        } else {
          sliceHasTight.append(sliceHasTight(index) || sliceHasTight(index + 1))
          sliceIsUnique.append(
            sliceIsUnique(index) && sliceIsUnique(index + 1) && !(sliceHasTight(index) && sliceHasTight(index + 1))
          )
        }
      }
      startIndex = endIndex
      endIndex = startIndex + numGroups
    }

    isUnique := sliceHasTight(sliceHasTight.length - 1) && sliceIsUnique(sliceIsUnique.length - 1)
  }
}

case class VertexIsUniqueTight(numEdges: Int) extends Component {
  require(numEdges > 0)

  val io = new Bundle {
    val tights = in(Vec.fill(numEdges)(Bool))
    val isUnique = out(Bool)
  }

  VertexIsUniqueTight.build(io.isUnique, io.tights)

}

// sbt 'testOnly microblossom.combinatorial.VertexIsUniqueTightTest'
class VertexIsUniqueTightTest extends AnyFunSuite {

  test("example") {
    val numEdges = 12
    Config.spinal().generateVerilog(VertexIsUniqueTight(numEdges))
  }

  test("logic validity") {
    val configurations = List(
      1, 2, 3, 4, 5, 6, 7, 8, 9
    )
    for (numEdges <- configurations) {
      Config.sim
        .compile(VertexIsUniqueTight(numEdges))
        .doSim("logic validity") { dut =>
          for (value <- Range(0, 1 << numEdges)) {
            var counter = 0
            for (index <- Range(0, numEdges)) {
              val bit = (value & (1 << index)) != 0
              dut.io.tights(index) #= bit
              if (bit) {
                counter = counter + 1
              }
            }
            sleep(1)
            val groundTruth = counter == 1
            assert(dut.io.isUnique.toBoolean == groundTruth, value)
          }
        }
    }
  }

}

// sbt 'testOnly microblossom.combinatorial.VertexIsUniqueTightDelayEstimation'
class VertexIsUniqueTightDelayEstimation extends AnyFunSuite {

  test("logic delay") {
    val configurations = List(
      (2, "code capacity 2 neighbors"), // 0.043ns
      (4, "code capacity 4 neighbors"), // 0.043ns
      (6, "phenomenological 6 neighbors"), // 0.043ns
      (12, "circuit-level 12 neighbors") // 0.67ns
    )
    for ((numEdges, name) <- configurations) {
      val timingReport = Vivado.reportTiming(VertexIsUniqueTight(numEdges))
      println(s"$name: ${timingReport.getPathDelaysExcludingIOWorst}ns")
    }
  }

}
