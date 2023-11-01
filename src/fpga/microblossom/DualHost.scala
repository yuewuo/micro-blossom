package microblossom

/*
 * A host of scala program that talks with a parent process through TCP
 *
 * The parent process needs to start a TCP server and listen to a specific address and port;
 * the address and port information is passed to the Scala program via command line arguments.
 * When started, the program will try to connect to the port and then fetch a JSON that
 * describes the decoding graph; it then constructs a dual accelerator and start simulator.
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

// sbt "runMain microblossom.DualHost localhost 4123"
object DualHost extends App {
  println(SpinalConfig.defaultTargetDirectory)
  if (args.length != 2) {
    println("usage: <address> <port>")
    sys.exit(1)
  }
  val hostname = args(0)
  val port = Integer.parseInt(args(1))
  val socket = new Socket(hostname, port)
  val workspacePath = "./simWorkspace/dualHost"
  try {
    val outStream = new PrintWriter(socket.getOutputStream, true)
    val inStream = new BufferedReader(new InputStreamReader(socket.getInputStream))

    // initial handshake and obtain a decoding graph
    outStream.println("DualHost v0.0.1, ask for decoding graph")
    var response = inStream.readLine()
    val graph = decode[SingleGraph](response) match {
      case Right(graph) => graph
      case Left(ex)     => throw ex
    }

    // construct and compile a dual accelerator for simulation
    val config = DualConfig(graph = graph, minimizeBits = false)
    config.sanityCheck()
    val simConfig = SimConfig
      .withConfig(Config.spinal)
      .workspacePath(workspacePath)
      .workspaceName(port.toString)

    var command = inStream.readLine()
    var withWaveform = false
    if (command == "with waveform") {
      simConfig.withFstWave
      withWaveform = true
    } else if (command == "no waveform") {} else {
      throw new IllegalArgumentException
    }

    simConfig
      .compile({
        val dut = DualAccelerator(config)
        dut.vertices.foreach(vertex => {
          vertex.io.simPublic()
        })
        dut.edges.foreach(edge => {
          edge.io.simPublic()
        })
        dut
      })
      .doSim("hosted") { dut =>
        outStream.println("simulation started")
        if (withWaveform) {
          println("view waveform: `gtkwave %s/%d/hosted.fst`".format(workspacePath, port))
        } else {
          println("waveform disabled")
        }

        dut.io.input.valid #= false
        dut.io.input.instruction #= 0
        dut.clockDomain.forkStimulus(period = 10)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.clockDomain.waitSampling()
        dut.io.input.valid #= true
        dut.io.input.instruction #= dut.config.instructionSpec.generateReset()

        dut.clockDomain.waitSampling()
        dut.io.input.valid #= false
        dut.io.input.instruction #= 0

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        def execute(instruction: Long) = {
          dut.io.input.valid #= true
          dut.io.input.instruction #= instruction
          dut.clockDomain.waitSampling()
          dut.io.input.valid #= false
          for (idx <- 0 to 2) { dut.clockDomain.waitSampling() }
          sleep(1)
        }

        // start hosting the commands
        breakable {
          while (true) {
            command = inStream.readLine()
            if (command == "quit") {
              println("requested quit, breaking...")
              break
            } else if (command == "reset()") {
              // TODO
            } else if (command.startsWith("set_speed(")) {
              val parameters = command.substring("set_speed(".length, command.length - 1).split(", ")
              assert(parameters.length == 2)
              val node = parameters(0).toInt
              val speed = if (parameters(1) == "Grow") { Speed.Grow }
              else if (parameters(1) == "Shrink") { Speed.Shrink }
              else { Speed.Stay }
              execute(dut.io.input.instruction.spec.generateSetSpeed(node, speed))
            } else if (command.startsWith("set_blossom(")) {
              val parameters = command.substring("set_blossom(".length, command.length - 1).split(", ")
              assert(parameters.length == 2)
              val node = parameters(0).toInt
              val blossom = parameters(1).toInt
              execute(dut.io.input.instruction.spec.generateSetBlossom(node, blossom))
            } else if (command == "find_obstacle()") {
              execute(dut.io.input.instruction.spec.generateFindObstacle())
              // TODO: send back the observed obstacle and the grown value
            }
          }
        }
      }

  } catch {
    case e: Exception => e.printStackTrace()
  } finally {
    socket.close()
    // also delete large verilator files, it's now attempted to delete on the Rust side
    // for (subfolder <- Seq("verilator", "rtl")) {
    //   val directory = new Directory(new File("%s/%d/%s".format(workspacePath, port, subfolder)))
    //   directory.deleteRecursively()
    // }
  }

}
