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
import java.util.concurrent.atomic._
import play.api.libs.json._
import io.circe.parser.decode
import scala.reflect.io.Directory
import scala.util.control.Breaks._
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
    val supportAddDefectVertex = readNamedValue("support_add_defect_vertex").toBoolean
    val injectRegistersJson = readNamedValue("inject_registers")
    val injectRegisters = decode[Seq[String]](injectRegistersJson) match {
      case Right(value) => value
      case Left(ex)     => throw ex
    }
    val clockDivideBy = readNamedValue("clock_divide_by").toInt

    // construct and compile a MicroBlossom module for simulation
    val config = DualConfig(
      graph = graph,
      contextDepth = contextDepth,
      broadcastDelay = broadcastDelay,
      convergecastDelay = convergecastDelay,
      conflictChannels = conflictChannels,
      supportAddDefectVertex = supportAddDefectVertex,
      injectRegisters = injectRegisters
    )
    config.sanityCheck()
    val simConfig = SimConfig
      .withConfig(Config.spinal())
      .workspacePath(workspacePath)
      .workspaceName(host_name)
    if (withWaveform) {
      simConfig.withFstWave
    } else {
      simConfig.allOptimisation
    }

    simConfig
      .compile({
        val busTypeFull = if (use64bus) {
          busType
        } else {
          s"${busType}Bus32"
        }
        val component: Component = MicroBlossomBusType.generateByName(busTypeFull, config, clockDivideBy)
        require(component.isInstanceOf[MicroBlossom[_, _]])
        val dut = component.asInstanceOf[MicroBlossom[IMasterSlave, BusSlaveFactoryDelayed]]
        if (withWaveform) {
          dut.simDual.simMakePublicSnapshot()
        }
        component
      })
      .doSim("hosted") { component =>
        val dut = component.asInstanceOf[MicroBlossom[IMasterSlave, BusSlaveFactoryDelayed]]

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

        val driver = dut.getSimDriver()
        driver.reset()

        dut.clockDomain.forkStimulus(period = 10)
        if (clockDivideBy > 1) {
          dut.dualClockDomain.forkStimulus(period = 10 * clockDivideBy)
        }

        val dumpJobs = ArrayBuffer[SimThread]()
        val endOfProgram = new AtomicBoolean(false)
        if (dumpDebuggerFiles) {
          println("dumping debugger files...")

          // dump the s0 interface
          {
            val interfaceSpec = dut.io.s0 match {
              case _: AxiLite4 => {
                Json.obj(
                  "interface" -> "AxiLite4",
                  "drive" -> Json.arr("awvalid", "awaddr", "awprot", "wvalid", "wdata", "wstrb", "bready"),
                  "read" -> Json.arr("awready", "wready", "bvalid", "bresp")
                )
              }
            }
            val axi4Dumper = fork {
              val filePath = "%s/%s/s0.debugger".format(workspacePath, host_name)
              val writer = new PrintWriter(new File(filePath))
              writer.println(Json.stringify(interfaceSpec))
              writer.flush()
              try {
                while (!endOfProgram.get()) {
                  dut.clockDomain.waitSampling()
                  sleep(1)
                  dut.io.s0 match {
                    case s0: AxiLite4 => {
                      writer.println(
                        Json.stringify(
                          Json.obj(
                            "awvalid" -> s0.aw.valid.toBoolean,
                            "awready" -> s0.aw.ready.toBoolean,
                            "awaddr" -> s0.aw.payload.addr.toBigInt,
                            "awprot" -> s0.aw.payload.prot.toBigInt,
                            "wvalid" -> s0.w.valid.toBoolean,
                            "wready" -> s0.w.ready.toBoolean,
                            "wdata" -> s0.w.payload.data.toBigInt,
                            "wstrb" -> s0.w.payload.strb.toBigInt,
                            "bvalid" -> s0.b.valid.toBoolean,
                            "bready" -> s0.b.ready.toBoolean,
                            "bresp" -> s0.b.payload.resp.toBigInt
                          )
                        )
                      )
                    }
                    case _ => throw new Exception(s"[error] axi4 dumper not supported for this interface")
                  }
                }
              } finally {
                writer.close()
              }
            }
            dumpJobs.append(axi4Dumper)
          }
        }

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }

        // start hosting the commands
        var maxGrowth = Long.MaxValue
        breakable {
          while (true) {
            val command = inStream.readLine()
            // println("[%d] %s".format(cycleCounter, command))
            if (command == "quit") {
              println("requested quit, aborting...")
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
            } else if (command.startsWith("snapshot(")) {
              dut.clockDomain.waitSampling()
              sleep(1)
              val parameters = command.substring("snapshot(".length, command.length - 1).split(", ")
              assert(parameters.length == 1)
              val abbrev = parameters(0).toBoolean
              outStream.println(dut.simDual.simSnapshot(abbrev).noSpacesSortKeys)
            } else {
              throw new Exception(s"[error] unknown command: $command")
            }
          }
        }

        for (idx <- 0 to 10) { dut.clockDomain.waitSampling() }
        endOfProgram.set(true)
        for (job <- dumpJobs) { job.join() }
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
