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

// sbt "runMain microblossom.DualHost localhost 4123 test"
object DualHost extends App {
  println(SpinalConfig.defaultTargetDirectory)
  if (args.length != 3) {
    println("usage: <address> <port> <host_name>")
    sys.exit(1)
  }
  val hostname = args(0)
  val port = Integer.parseInt(args(1))
  val host_name = args(2)
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
      .withConfig(Config.spinal())
      .workspacePath(workspacePath)
      .workspaceName(host_name)

    var command = inStream.readLine()
    var withWaveform = false
    if (command == "with waveform") {
      simConfig.withFstWave
      withWaveform = true
    } else if (command == "no waveform") {
      simConfig.allOptimisation
    } else {
      throw new IllegalArgumentException
    }

    simConfig
      .compile({
        val dut = DualAccelerator(config)
        if (withWaveform) {
          dut.simMakePublicSnapshot()
        }
        dut
      })
      .doSim("hosted") { dut =>
        val ioConfig = dut.ioConfig
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

        dut.io.input.valid #= false
        dut.io.input.instruction #= 0
        dut.clockDomain.forkStimulus(period = 10)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.simExecute(ioConfig.instructionSpec.generateReset())

        // start hosting the commands
        var maxGrowth = Long.MaxValue
        breakable {
          while (true) {
            command = inStream.readLine()
            // println("[%d] %s".format(cycleCounter, command))
            if (command == "quit") {
              println("requested quit, breaking...")
              break
            } else if (command == "reset()") {
              dut.simExecute(ioConfig.instructionSpec.generateReset())
            } else if (command.startsWith("set_speed(")) {
              val parameters = command.substring("set_speed(".length, command.length - 1).split(", ")
              assert(parameters.length == 2)
              val node = parameters(0).toInt
              val speed = if (parameters(1) == "Grow") { Speed.Grow }
              else if (parameters(1) == "Shrink") { Speed.Shrink }
              else { Speed.Stay }
              dut.simExecute(ioConfig.instructionSpec.generateSetSpeed(node, speed))
            } else if (command.startsWith("set_blossom(")) {
              val parameters = command.substring("set_blossom(".length, command.length - 1).split(", ")
              assert(parameters.length == 2)
              val node = parameters(0).toInt
              val blossom = parameters(1).toInt
              dut.simExecute(ioConfig.instructionSpec.generateSetBlossom(node, blossom))
            } else if (command.startsWith("set_maximum_growth(")) {
              val parameters = command.substring("set_maximum_growth(".length, command.length - 1).split(", ")
              assert(parameters.length == 1)
              maxGrowth = parameters(0).toLong
            } else if (command == "find_obstacle()") {
              val (obstacle, grown) = dut.simFindObstacle(maxGrowth)
              maxGrowth -= grown
              val reader = ObstacleReader(ioConfig, obstacle)
              if (reader.rspCode == RspCode.NonZeroGrow) {
                outStream.println(
                  "NonZeroGrow(%d), %d".format(
                    if (reader.length == ioConfig.LengthNone) { Int.MaxValue }
                    else { reader.length },
                    grown
                  )
                )
              } else if (reader.rspCode == RspCode.Conflict) {
                val (node1, node2, touch1, touch2, vertex1, vertex2) = if (reader.field2 == ioConfig.IndexNone) {
                  (reader.field1, reader.field2, reader.field3, reader.field4, reader.field5, reader.field6)
                } else {
                  (reader.field2, reader.field1, reader.field4, reader.field3, reader.field6, reader.field5)
                }
                assert(node1 != ioConfig.IndexNone)
                assert(touch1 != ioConfig.IndexNone)
                outStream.println(
                  "Conflict(%d, %d, %d, %d, %d, %d), %d"
                    .format(
                      node1,
                      if (node2 == ioConfig.IndexNone) { Int.MaxValue }
                      else { node2 },
                      touch1,
                      if (touch2 == ioConfig.IndexNone) { Int.MaxValue }
                      else { touch2 },
                      vertex1,
                      vertex2,
                      grown
                    )
                )
              } else {
                outStream.println("illegal obstacle arguments")
                throw new IllegalArgumentException
              }
            } else if (command.startsWith("add_defect(")) {
              val parameters = command.substring("add_defect(".length, command.length - 1).split(", ")
              assert(parameters.length == 2)
              val vertex = parameters(0).toInt
              val node = parameters(1).toInt
              dut.simExecute(ioConfig.instructionSpec.generateAddDefect(vertex, node))
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
    // also delete large verilator files, it's now attempted to delete on the Rust side
    // for (subfolder <- Seq("verilator", "rtl")) {
    //   val directory = new Directory(new File("%s/%d/%s".format(workspacePath, host_name, subfolder)))
    //   directory.deleteRecursively()
    // }
  }

}
