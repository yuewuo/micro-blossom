package microblossom

/*
 * Same as DualHost.scala, but using MicroBlossomLooper class with streaming interface
 *
 */

import java.io._
import java.net._
import util._
import spinal.core._
import spinal.core.sim._
import io.circe.parser.decode
import scala.reflect.io.Directory
import scala.util.control.Breaks._
import modules._

// sbt "runMain microblossom.LooperHost localhost 4123 test"
object LooperHost extends SimulationTcpHost("LooperHost") {
  try {

    // initial handshake and obtain a decoding graph
    outStream.println(s"${simulationName} v0.0.1, ask for decoding graph")
    val emuConfig = SimulationConfig.readFromStream(inStream)
    val config = emuConfig.dualConfig
    val simConfig = emuConfig.simConfig(workspacePath, name)

    simConfig
      .compile({
        val dut = MicroBlossomLooper(config)
        if (emuConfig.withWaveform) {
          dut.microBlossom.simMakePublicSnapshot()
        }
        dut
      })
      .doSim("hosted") { dut =>
        simulationStarted()

        var cycleCounter = 0L
        dut.clockDomain.onActiveEdges { cycleCounter += 1 }

        dut.clockDomain.forkStimulus(period = 10)
        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        // start hosting the commands
        breakable {
          while (true) {
            val command = inStream.readLine()
            // println("[%d] %s".format(cycleCounter, command))
            if (command == "quit") {
              println("requested quit, aborting...")
              break
            }
          }
        }

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

      }

  } catch {
    case e: Exception => e.printStackTrace()
  } finally {
    socket.close()
    if (cleanFileOnDisconnect) {
      for (subfolder <- Seq("verilator", "rtl")) {
        val directory = new Directory(new File("%s/%d/%s".format(workspacePath, name, subfolder)))
        directory.deleteRecursively()
      }
    }
  }
}
