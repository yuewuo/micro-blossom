package microblossom

/*
 * Same as DualHost.scala, but using MicroBlossomLooper class with streaming interface
 *
 */

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import java.io._
import java.net._
import util._
import spinal.core._
import spinal.core.sim._
import io.circe.parser.decode
import scala.reflect.io.Directory
import scala.util.control.Breaks._
import types._
import modules._
import java.lang.module.ModuleDescriptor.Exports

@ConfiguredJsonCodec
case class LooperHostInputData(
    var instruction: Long,
    var contextId: Int,
    var instructionId: Int,
    var maximumGrowth: Int
)

object LooperHostInputData {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

@ConfiguredJsonCodec
case class LooperHostOutData(
    var contextId: Int,
    var instructionId: Int,
    var maxGrowable: Int,
    var conflict: DataConflict,
    var grown: Int
)

object LooperHostOutData {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}

// sbt "runMain microblossom.LooperHost localhost 4123 test"
object LooperHost extends SimulationTcpHost("LooperHost") {
  try {

    // initial handshake and obtain a decoding graph
    outStream.println(s"${simulationName} v0.0.1, ask for decoding graph")
    val emuConfig = SimulationConfig.readFromStream(inStream)
    val config = emuConfig.dualConfig
    val simConfig = emuConfig.simConfig(workspacePath, name)
    val clientSpec = DualConfig().instructionSpec // client side uses the default 32 bit instruction format

    simConfig
      .compile({
        val dut = MicroBlossomLooper(config)
        if (emuConfig.withWaveform) {
          dut.simMakePublicSnapshot()
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
            } else if (command.startsWith("execute: ")) {
              var json_content = command.substring("execute: ".length, command.length)
              var inputData = decode[LooperHostInputData](json_content) match {
                case Right(inputData) => inputData
                case Left(ex)         => throw ex
              }
              println(clientSpec.format(inputData.instruction))
              // adapt instruction width
              val instruction = config.instructionSpec.from(inputData.instruction, clientSpec)
              println(config.instructionSpec.format(instruction))
              dut.simExecute(instruction, inputData.contextId, inputData.maximumGrowth)

              throw new Exception("unimplemented")
            } else if (command.startsWith("snapshot(")) {
              val parameters = command.substring("snapshot(".length, command.length - 1).split(", ")
              assert(parameters.length == 1)
              val abbrev = parameters(0).toBoolean
              outStream.println(dut.simSnapshot(abbrev).noSpacesSortKeys)
            } else {
              println("[error] unknown command: %s".format(command))
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
