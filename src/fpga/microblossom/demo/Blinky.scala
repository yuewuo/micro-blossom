package microblossom.demo

import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom.Config

class Blinky extends Component {
  val io = new Bundle {
    val led = out Bool
  }

  val led_reg = Reg(Bool()) init False
  io.led := led_reg

  led_reg := !led_reg

}

// sbt "runMain microblossom.demo.BlinkyVerilog"
object BlinkyVerilog extends App {
  Config.spinal.generateVerilog(new Blinky())
}

// sbt "runMain microblossom.demo.BlinkyTestA" && gtkwave simWorkspace/Blinky/testA.fst
object BlinkyTestA extends App {
  Config.sim.compile(new Blinky()).doSim("testA") { dut =>
    dut.clockDomain.forkStimulus(10)
    sleep(1000)
  }
}

// sbt "runMain microblossom.demo.BlinkyTestB" && gtkwave simWorkspace/Blinky/testB.fst
object BlinkyTestB extends App {
  Config.sim.compile(new Blinky()).doSim("testB") { dut =>
    dut.clockDomain.forkStimulus(10)
    sleep(200)
  }
}
