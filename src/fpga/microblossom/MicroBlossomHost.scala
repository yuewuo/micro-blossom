package microblossom

/*
 * # Micro Blossom Host with AXI4 simulation
 *
 * Similar to DualHost, it communicates with a parent process through TCP and provides read/write
 * to the underlying AXI4 interface.
 *
 */

import io.circe.syntax._
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
object MicroBlossomHost extends SimulationTcpHost("MicroBlossomHost") {

  // initial handshake and obtain a decoding graph
  outStream.println(s"${simulationName} v0.0.1, ask for decoding graph")
  val emuConfig = SimulationConfig.readFromStream(inStream)
  val config = emuConfig.dualConfig
  val simConfig = emuConfig.simConfig(workspacePath, name)
  val clientSpec = DualConfig().instructionSpec // client side uses the default 32 bit instruction format
  val dumpJobs = ArrayBuffer[SimThread]()
  val endOfProgram = new AtomicBoolean(false)

  try {
    simConfig
      .compile({
        val busTypeFull = if (emuConfig.use64bus) { emuConfig.busType }
        else { s"${emuConfig.busType}Bus32" }
        val component: Component = MicroBlossomBusType.generateByName(busTypeFull, config, emuConfig.clockDivideBy)
        require(component.isInstanceOf[MicroBlossomBus[_, _]])
        val dut = component.asInstanceOf[MicroBlossomBus[IMasterSlave, BusSlaveFactoryDelayed]]
        if (emuConfig.withWaveform) {
          dut.simMakePublicSnapshot()
        }
        dut.simMakePublicPreMatching()
        dut
      })
      .doSim("hosted") { dut =>
        simulationStarted()

        var cycleCounter = 0L
        dut.clockDomain.onActiveEdges { cycleCounter += 1 }

        val driver = dut.getSimDriver()
        driver.reset()

        dut.clockDomain.forkStimulus(period = 10)
        dut.slowClockDomain.forkStimulus(period = (10 * emuConfig.clockDivideBy).toInt)

        debuggerDump(dut)

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
              val parameters = command.substring("snapshot(".length, command.length - 1).split(", ")
              assert(parameters.length == 1)
              val abbrev = parameters(0).toBoolean
              outStream.println(dut.simSnapshot(abbrev).noSpacesSortKeys)
            } else if (command == "pre_matchings()") {
              outStream.println(dut.simPreMatchings().asJson.noSpacesSortKeys)
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
    if (cleanFileOnDisconnect) {
      for (subfolder <- Seq("verilator", "rtl")) {
        val directory = new Directory(new File("%s/%d/%s".format(workspacePath, name, subfolder)))
        directory.deleteRecursively()
      }
    }
  }

  def debuggerDump(dut: MicroBlossomBus[IMasterSlave, BusSlaveFactoryDelayed]) = {
    if (emuConfig.dumpDebuggerFiles) {
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
          val filePath = "%s/s0.debugger".format(workspacePath)
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
  }

}
