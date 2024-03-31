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
    val supportOffloading = readNamedValue("support_offloading").toBoolean
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
      supportOffloading = supportOffloading,
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
              case s0: AxiLite4 => {
                Json.obj(
                  "interface" -> "AxiLite4",
                  "drive" -> Json.arr(
                    "awvalid",
                    "awaddr",
                    "awprot",
                    "wvalid",
                    "wdata",
                    "wstrb",
                    "bready",
                    "arvalid",
                    "araddr",
                    "arprot",
                    "rready"
                  ),
                  "drive_width" -> Json.arr(
                    1,
                    s0.aw.addr.getBitsWidth,
                    s0.aw.prot.getBitsWidth,
                    1,
                    s0.w.data.getBitsWidth,
                    s0.w.strb.getBitsWidth,
                    1,
                    1,
                    s0.ar.addr.getBitsWidth,
                    s0.ar.prot.getBitsWidth,
                    1
                  ),
                  "read" -> Json.arr("awready", "wready", "bvalid", "bresp", "arready", "rvalid", "rdata", "rresp"),
                  "read_width" -> Json.arr(
                    1,
                    1,
                    1,
                    s0.b.resp.getBitsWidth,
                    1,
                    1,
                    s0.r.data.getBitsWidth,
                    s0.r.resp.getBitsWidth
                  )
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
                            "drive" -> Json.arr(
                              s0.aw.valid.toBigInt, // awvalid
                              s0.aw.payload.addr.toBigInt, // awaddr
                              s0.aw.payload.prot.toBigInt, // awprot
                              s0.w.valid.toBigInt, // wvalid
                              s0.w.payload.data.toBigInt, // wdata
                              s0.w.payload.strb.toBigInt, // wstrb
                              s0.b.ready.toBigInt, // bready
                              s0.ar.valid.toBigInt, // arvalid
                              s0.ar.payload.addr.toBigInt, // araddr
                              s0.ar.payload.prot.toBigInt, // arprot
                              s0.r.ready.toBigInt // rready
                            ),
                            "read" -> Json.arr(
                              s0.aw.ready.toBigInt, // awready
                              s0.w.ready.toBigInt, // wready
                              s0.b.valid.toBigInt, // bvalid
                              s0.b.payload.resp.toBigInt, // bresp
                              s0.ar.ready.toBigInt, // arready
                              s0.r.valid.toBigInt, // rvalid
                              s0.r.payload.data.toBigInt, // rdata
                              s0.r.payload.resp.toBigInt // rresp
                            )
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
