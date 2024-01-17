package microblossom.util

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.amba4.axilite.sim._
import spinal.lib.sim.StreamDriver
import spinal.core.sim._
import microblossom._
import microblossom.util._
import org.scalatest.funsuite.AnyFunSuite
import scala.collection.mutable

case class AxiLite4DriverDeterministic(axi: AxiLite4, clockDomain: ClockDomain) {
  def reset(): Unit = {
    axi.aw.valid #= false
    axi.w.valid #= false
    axi.ar.valid #= false
    axi.r.ready #= true
    axi.b.ready #= true

  }

  reset()

  def read(address: BigInt): BigInt = {
    axi.ar.payload.prot.assignBigInt(6)

    axi.ar.valid #= true
    axi.ar.payload.addr #= address
    axi.ar.prot #= 0

    axi.r.ready #= true

    clockDomain.waitSamplingWhere(axi.ar.ready.toBoolean)

    axi.ar.valid #= false

    clockDomain.waitSamplingWhere(axi.r.valid.toBoolean)

    axi.r.ready #= false

    axi.r.payload.data.toBigInt
  }

  val awQueue = mutable.Queue[() => Unit]()
  val awDriver = StreamDriverDeterministic(axi.aw, clockDomain) { _ =>
    if (awQueue.nonEmpty) {
      awQueue.dequeue().apply()
      true
    } else {
      false
    }
  }

  val wQueue = mutable.Queue[() => Unit]()
  val wDriver = StreamDriverDeterministic(axi.w, clockDomain) { _ =>
    if (wQueue.nonEmpty) {
      wQueue.dequeue().apply()
      true
    } else {
      false
    }
  }

  def write(address: BigInt, data: BigInt): Unit = {
    awQueue.enqueue { () =>
      axi.aw.addr #= address
      axi.aw.prot #= 0
    }

    wQueue.enqueue { () =>
      axi.w.data #= data
      axi.w.strb #= (BigInt(1) << axi.config.bytePerWord) - 1
    }

    clockDomain.waitSamplingWhere(wQueue.isEmpty && awQueue.isEmpty)
    clockDomain.waitSamplingWhere(axi.b.ready.toBoolean && axi.b.valid.toBoolean)
    // reset values so that waveform is simpler
    axi.aw.addr #= 0
    axi.w.data #= 0
    axi.w.strb #= 0

  }

}
