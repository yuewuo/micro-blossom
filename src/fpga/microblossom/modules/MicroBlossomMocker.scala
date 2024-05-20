package microblossom.modules

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite
import scala.util.control.Breaks._
import scala.collection.mutable.ArrayBuffer
import scala.collection.mutable.Map

case class MicroBlossomMocker(config: DualConfig, ioConfig: DualConfig = DualConfig()) extends Component {
  ioConfig.contextDepth = config.contextDepth

  val io = new Bundle {
    val message = in(BroadcastMessage(ioConfig, explicitReset = false))

    val maxGrowable = out(ConvergecastMaxGrowable(ioConfig.weightBits))
    val conflict = out(ConvergecastConflict(ioConfig.vertexBits))
  }

  val validSR = ShiftRegister(Bool(), depth = config.readLatency, initFunc = (x: Bool) => x.init(False))
  validSR.io.input := io.message.valid

  when(validSR.io.output) {
    io.conflict.valid := True
    io.conflict.node1 := B("d1").resized
    io.conflict.node2 := B("d2").resized
    io.conflict.touch1 := B("d3").resized
    io.conflict.touch2 := B("d4").resized
    io.conflict.vertex1 := B("d5").resized
    io.conflict.vertex2 := B("d6").resized
    io.maxGrowable.length := U(0)
  } otherwise {
    io.conflict.valid := False
    io.conflict.node1 := B("d0").resized
    io.conflict.node2 := B("d0").resized
    io.conflict.touch1 := B("d0").resized
    io.conflict.touch2 := B("d0").resized
    io.conflict.vertex1 := B("d0").resized
    io.conflict.vertex2 := B("d0").resized
    io.maxGrowable.length := io.maxGrowable.length.maxValue
  }

}

// sbt 'testOnly *MicroBlossomMockerTest'
class MicroBlossomMockerTest extends AnyFunSuite {

  test("logic_validity") {
    for (delay <- 0 to 3) {

      val config = DualConfig(
        filename = "./resources/graphs/example_code_capacity_d3.json",
        broadcastDelay = delay
      )

      Config.sim
        .compile(MicroBlossomMocker(config))
        .doSim("logic_validity") { dut =>
          dut.clockDomain.forkStimulus(period = 10)

          dut.io.message.valid #= false
          for (idx <- 0 until 10) { dut.clockDomain.waitSampling() }

          dut.io.message.valid #= true
          sleep(2)
          for (idx <- 0 until delay + 1) {
            println(s"delay: $delay, idx: $idx, valid: ${dut.io.conflict.valid.toBoolean}")
            assert(dut.io.conflict.valid.toBoolean == (idx == delay))
            dut.clockDomain.waitSampling()
            dut.io.message.valid #= false
            sleep(2)
          }

          for (idx <- 0 until 10) {
            assert(dut.io.conflict.valid.toBoolean == false)
            dut.clockDomain.waitSampling()
          }
        }

    }
  }

}
