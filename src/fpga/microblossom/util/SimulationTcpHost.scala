package microblossom.util

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
import java.util.concurrent.atomic._
import play.api.libs.json._
import io.circe.parser.decode
import scala.reflect.io.Directory
import scala.util.control.Breaks._
import microblossom._
import microblossom.util._
import microblossom.modules._
import microblossom.driver._
import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axi._
import spinal.lib.bus.amba4.axilite._
import spinal.sim._
import spinal.core.sim._
import spinal.lib.bus.misc._
import scala.collection.mutable.ArrayBuffer

class SimulationTcpHost(val simulationName: String, val cleanFileOnDisconnect: Boolean = false) extends App {

  if (args.length != 3) {
    println("usage: <address> <port> <name>")
    sys.exit(1)
  }
  val hostname = args(0)
  val port = Integer.parseInt(args(1))
  val name = args(2)
  val socket = new Socket(hostname, port)
  val workspacePath = s"./simWorkspace/${simulationName}"
  val outStream = new PrintWriter(socket.getOutputStream, true)
  val inStream = new BufferedReader(new InputStreamReader(socket.getInputStream))

  // must call this function after the simulator is compiled
  def simulationStarted() = {
    outStream.println("simulation started")
  }
}

case class SimulationConfig(
    var graph: SingleGraph,
    var withWaveform: Boolean,
    var dumpDebuggerFiles: Boolean,
    var busType: String,
    val use64bus: Boolean,
    val contextDepth: Int,
    val broadcastDelay: Int,
    val convergecastDelay: Int,
    val conflictChannels: Int,
    val hardCodeWeights: Boolean,
    val supportAddDefectVertex: Boolean,
    val supportOffloading: Boolean,
    val supportLayerFusion: Boolean,
    val injectRegisters: Seq[String],
    val clockDivideBy: Int
) {
  def dualConfig = {
    val config = DualConfig(
      graph = graph,
      contextDepth = contextDepth,
      broadcastDelay = broadcastDelay,
      convergecastDelay = convergecastDelay,
      conflictChannels = conflictChannels,
      hardCodeWeights = hardCodeWeights,
      supportAddDefectVertex = supportAddDefectVertex,
      supportOffloading = supportOffloading,
      injectRegisters = injectRegisters
    )
    config.sanityCheck()
    config
  }

  def simConfig(workspacePath: String, name: String) = {
    val simConfig = SimConfig
      .withConfig(Config.spinal())
      .workspacePath(workspacePath)
      .workspaceName(name)
    if (withWaveform) {
      println("view waveform: `gtkwave %s/%s/hosted.fst`".format(workspacePath, name))
      simConfig.withFstWave
    } else {
      println("waveform disabled")
      simConfig.allOptimisation
    }
  }
}

object SimulationConfig {
  def readFromStream(inStream: BufferedReader): SimulationConfig = {
    var response = inStream.readLine()
    val graph = decode[SingleGraph](response) match {
      case Right(graph) => graph
      case Left(ex)     => throw ex
    }

    def readNamedValue(name: String): String = {
      val command = inStream.readLine()
      println(command)
      assert(command.startsWith(s"$name = "))
      command.substring(s"$name = ".length, command.length)
    }

    val withWaveform = readNamedValue("with_waveform").toBoolean
    val dumpDebuggerFiles = readNamedValue("dump_debugger_files").toBoolean
    val busType = decode[String](readNamedValue("bus_type")) match {
      case Right(value) => value
      case Left(ex)     => throw ex
    }
    val use64bus = readNamedValue("use_64_bus").toBoolean
    val contextDepth = readNamedValue("context_depth").toInt
    val broadcastDelay = readNamedValue("broadcast_delay").toInt
    val convergecastDelay = readNamedValue("convergecast_delay").toInt
    val conflictChannels = readNamedValue("conflict_channels").toInt
    val hardCodeWeights = readNamedValue("hard_code_weights").toBoolean
    val supportAddDefectVertex = readNamedValue("support_add_defect_vertex").toBoolean
    val supportOffloading = readNamedValue("support_offloading").toBoolean
    val supportLayerFusion = readNamedValue("support_layer_fusion").toBoolean
    val injectRegistersJson = readNamedValue("inject_registers")
    val injectRegisters = decode[Seq[String]](injectRegistersJson) match {
      case Right(value) => value
      case Left(ex)     => throw ex
    }
    val clockDivideBy = readNamedValue("clock_divide_by").toInt
    SimulationConfig(
      graph,
      withWaveform,
      dumpDebuggerFiles,
      busType,
      use64bus,
      contextDepth,
      broadcastDelay,
      convergecastDelay,
      conflictChannels,
      hardCodeWeights,
      supportAddDefectVertex,
      supportOffloading,
      supportLayerFusion,
      injectRegisters,
      clockDivideBy
    )
  }
}
