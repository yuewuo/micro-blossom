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
object LooperHost extends EmulationTcpHost("LooperHost") {}
