package microblossom.modules

/*
 * # Emulate Load Stall
 *
 * When loading syndrome from real quantum control system, the data becomes available only after
 * certain amount of time. For example, to load the 5-th layer of syndrome, it's only available
 * after 5us. Trying to load future syndrome should stall the system until it becomes available.
 *
 * This module emulates this behavior, by issuing a stall signal if the time has not arrived yet.
 *
 */
import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import spinal.core._
import spinal.lib._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.types._
import microblossom.util.Vivado
import org.scalatest.funsuite.AnyFunSuite
import scala.util.control.Breaks._
import scala.collection.mutable.ArrayBuffer
import scala.collection.mutable.Map

case class LoadStallEmulator(config: DualConfig, timeBit: Int = 64, intervalBits: Int = 32) extends Component {
  val io = new Bundle {
    val currentTime = in UInt (timeBit bits)
    val startTime = in UInt (timeBit bits)
    val interval = in UInt (intervalBits bits)
    val layerId = in UInt (config.layerIdBits bits)
    val isStall = out Bool ()
  }

  val readyTime = UInt(timeBit bits)
  readyTime := io.startTime + (io.interval * io.layerId).resized
  io.isStall := io.currentTime < readyTime

}
