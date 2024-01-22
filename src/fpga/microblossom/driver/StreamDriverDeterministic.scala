package microblossom.util

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.amba4.axilite.sim._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import org.scalatest.funsuite.AnyFunSuite
import scala.collection.mutable

object StreamDriverDeterministic {
  def apply[T <: Data](stream: Stream[T], clockDomain: ClockDomain)(driver: (T) => Boolean) =
    new StreamDriverDeterministic(stream, clockDomain, driver)

  def queue[T <: Data](stream: Stream[T], clockDomain: ClockDomain) = {
    val cmdQueue = mutable.Queue[(T) => Unit]()
    val driver = StreamDriverDeterministic(stream, clockDomain) { p =>
      if (cmdQueue.isEmpty) false
      else {
        cmdQueue.dequeue().apply(p)
        true
      }
    }
    (driver, cmdQueue)
  }
}

class StreamDriverDeterministic[T <: Data](stream: Stream[T], clockDomain: ClockDomain, var driver: (T) => Boolean) {

  var state = 1
  val validProxy = stream.valid.simProxy()
  validProxy #= false

  val readyProxy = stream.ready.simProxy()

  def fsm(): Unit = {
    state match {
      case 1 => {
        if (driver(stream.payload)) {
          validProxy #= true
          state += 1
        }
      }
      case 2 => {
        if (readyProxy.toBoolean) {
          validProxy #= false
          state = 1
        }
      }
    }
  }
  clockDomain.onSamplings(fsm)

  def reset() = {
    state = 1
    stream.valid #= false
  }
}
