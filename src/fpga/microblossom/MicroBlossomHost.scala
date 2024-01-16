package microblossom

/*
 * # Micro Blossom Host with AXI4 emulation
 *
 * Similar to DualHost, it communicates with a parent process through TCP and provides read/write
 * to the underlying AXI4 interface.
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

// sbt "runMain microblossom.MicroBlossomHost localhost 4123 test"
object MicroBlossomHost extends App {
  println(SpinalConfig.defaultTargetDirectory)
  if (args.length != 3) {
    println("usage: <address> <port> <host_name>")
    sys.exit(1)
  }
  val hostname = args(0)
  val port = Integer.parseInt(args(1))
  val host_name = args(2)
  val socket = new Socket(hostname, port)
  val workspacePath = "./simWorkspace/MicroBlossomHost"
  try {
    val outStream = new PrintWriter(socket.getOutputStream, true)
    val inStream = new BufferedReader(new InputStreamReader(socket.getInputStream))

    // initial handshake and obtain a decoding graph
    outStream.println("MicroBlossomHost v0.0.1, ask for decoding graph")
    var response = inStream.readLine()
    val graph = decode[SingleGraph](response) match {
      case Right(graph) => graph
      case Left(ex)     => throw ex
    }

    // construct and compile a MicroBlossom module for simulation
    val config = DualConfig(graph = graph)
    config.sanityCheck()
    val simConfig = SimConfig
      .withConfig(Config.spinal())
      .workspacePath(workspacePath)
      .workspaceName(host_name)

    var command = inStream.readLine()
    var withWaveform = false
    command match {
      case "with waveform" => {
        simConfig.withFstWave
        withWaveform = true
      }
      case "no waveform" => simConfig.allOptimisation
      case _             => throw new IllegalArgumentException
    }

    command = inStream.readLine()
    var use64bus = true
    command match {
      case "64 bits bus" => use64bus = true
      case "32 bits bus" => use64bus = false
      case _             => throw new IllegalArgumentException
    }

    simConfig
      .compile({
        val dut = if (use64bus) {
          MicroBlossomAxiLite4(config)
        } else {
          MicroBlossomAxiLite4Bus32(config)
        }
        if (withWaveform) {
          // dut.simMakePublicSnapshot()  // TODO: implement
        }
        dut
      })
      .doSim("hosted") { dut =>
        outStream.println("simulation started")
        if (withWaveform) {
          println("view waveform: `gtkwave %s/%s/hosted.fst`".format(workspacePath, host_name))
        } else {
          println("waveform disabled")
        }

        var cycleCounter = 0L
        dut.clockDomain.onActiveEdges {
          cycleCounter += 1
        }

        dut.clockDomain.forkStimulus(period = 10)
        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        val driver = AxiLite4TypedDriver(dut.io.s0, dut.clockDomain)

        // start hosting the commands
        var maxGrowth = Long.MaxValue
        breakable {
          while (true) {
            command = inStream.readLine()
            // println("[%d] %s".format(cycleCounter, command))
            if (command == "quit") {
              println("requested quit, breaking...")
              break
            } else if (command.startsWith("read(")) {
              // format: read(numBytes, address)
              // example: read(64, 0)
              val parameters = command.substring("read(".length, command.length - 1).split(", ")
              assert(parameters.length == 2)
              val numBytes = parameters(0).toInt
              val address = BigInt(parameters(1))
              val data = driver.readBytes(address, numBytes)
              outStream.println(s"$data")
            } else if (command.startsWith("write(")) {
              // format: write(numBytes, address, data)
              // example: write(64, 0, 123)
              val parameters = command.substring("write(".length, command.length - 1).split(", ")
              assert(parameters.length == 3)
              val numBytes = parameters(0).toInt
              val address = BigInt(parameters(1))
              val data = BigInt(parameters(2))
              driver.writeBytes(address, data, numBytes)
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
    // also delete large verilator files, it's now attempted to delete on the Rust side
    // for (subfolder <- Seq("verilator", "rtl")) {
    //   val directory = new Directory(new File("%s/%d/%s".format(workspacePath, host_name, subfolder)))
    //   directory.deleteRecursively()
    // }
  }

}
