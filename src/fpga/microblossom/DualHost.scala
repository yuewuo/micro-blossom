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
  val workspacePath = System.getenv().getOrDefault("SPINALSIM_WORKSPACE", "./simWorkspace") + "/dualHost"
  try {
    val outStream = new PrintWriter(socket.getOutputStream, true)
    val inStream = new BufferedReader(new InputStreamReader(socket.getInputStream))

    // initial handshake and obtain a decoding graph
    outStream.println("DualHost v0.0.1, ask for decoding graph")
    val response = inStream.readLine()
    val graph = decode[SingleGraph](response) match {
      case Right(graph) => graph
      case Left(ex)     => throw ex
    }

    // construct and compile a dual accelerator for simulation
    val config = DualConfig(graph = graph, minimizeBits = false)
    config.sanityCheck()
    SimConfig
      .withConfig(Config.spinal)
      .workspacePath(workspacePath)
      .workspaceName(port.toString)
      .withFstWave
      .compile({
        val dut = DualAccelerator(config)
        dut.vertices.foreach(vertex => {
          vertex.io.simPublic()
        })
        dut
      })
      .doSim("hosted") { dut =>
        outStream.println("simulation started")
        outStream.println("view waveform: `gtkwave %s/%d/hosted.fst`".format(workspacePath, port))

        dut.io.valid #= false
        dut.clockDomain.forkStimulus(period = 10)

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        dut.clockDomain.waitSampling()
        dut.io.valid #= true
        dut.io.instruction #= dut.config.instructionSpec.generateReset()

        dut.clockDomain.waitSampling()
        dut.io.valid #= false

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
      }

  } catch {
    case e: Exception => e.printStackTrace()
  } finally {
    socket.close()
    // also delete large verilator files
    for (subfolder <- Seq("verilator", "rtl")) {
      val directory = new Directory(new File("%s/%d/%s".format(workspacePath, port, subfolder)))
      directory.deleteRecursively()
    }
  }

}
