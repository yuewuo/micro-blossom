package microblossom.driver

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axi._
import spinal.lib.bus.amba4.axi.sim._
import spinal.lib.bus.misc._
import spinal.core.sim._
import spinal.lib.sim._
import microblossom._
import microblossom.util._
import scala.collection.mutable
import org.scalatest.funsuite.AnyFunSuite

case class Axi4TypedDriver(axi: Axi4, clockDomain: ClockDomain) extends TypedDriver {
  val busConfig = axi.config
  require(busConfig.dataWidth == 64, "only 64 bits bus is supported")

  val aw = axi.aw
  val w = axi.w
  val b = axi.b
  val ar = axi.ar
  val r = axi.r

  val awQueue = mutable.Queue[() => Unit]()
  val wQueue = mutable.Queue[() => Unit]()
  val bQueue = mutable.Queue[() => Unit]()
  def writePending = bQueue.nonEmpty

  def awCallback(aw: Axi4Aw) = {
    if (awQueue.nonEmpty) { awQueue.dequeue().apply(); true }
    else false
  }
  def wCallback(w: Axi4W) = {
    if (wQueue.nonEmpty) { wQueue.dequeue().apply(); true }
    else false
  }

  val arQueue = mutable.Queue[() => Unit]()
  val rQueue = mutable.Queue[() => Unit]()
  def readPending = rQueue.nonEmpty

  def arCallback(ar: Axi4Ar) = {
    if (arQueue.nonEmpty) { arQueue.dequeue().apply(); true }
    else false
  }

  val awDriver = StreamDriverDeterministic(aw, clockDomain)(awCallback)
  val wDriver = StreamDriverDeterministic(w, clockDomain)(wCallback)
  val arDriver = StreamDriverDeterministic(ar, clockDomain)(arCallback)
  // val awDriver = StreamDriver(aw, clockDomain)(awCallback)
  // val wDriver = StreamDriver(w, clockDomain)(wCallback)
  // val arDriver = StreamDriver(ar, clockDomain)(arCallback)

  val writeRspMonitor = StreamMonitor(b, clockDomain) { _ =>
    if (bQueue.nonEmpty) { bQueue.dequeue()() }
  }
  val readRspMonitor = StreamMonitor(r, clockDomain) { _ =>
    if (rQueue.nonEmpty) { rQueue.dequeue()() }
  }

  def reset(): Unit = {
    ar.valid #= false
    w.valid #= false
    b.ready #= true
    ar.valid #= false
    r.ready #= true
    wQueue.clear()
    bQueue.clear()
    awQueue.clear()
    awDriver.reset()
    wDriver.reset()
    rQueue.clear()
    arQueue.clear()
    arDriver.reset()
  }

  reset()

  def readBytes(address: BigInt, numBytes: Int): BigInt = {
    assert(numBytes == 1 || numBytes == 2 || numBytes == 4 || numBytes == 8)
    assert(address % numBytes == 0, "address is not aligned")

    val byteOffset = (address & (busConfig.bytePerWord - 1)).toInt
    val alignedAddress = address - byteOffset

    var readValue: BigInt = null
    rQueue.enqueue { () =>
      if (busConfig.useResp) assert(r.resp.toInt == 0)
      if (busConfig.useLast) assert(r.last.toBoolean == true)
      readValue = r.data.toBigInt
    }

    arQueue.enqueue { () =>
      ar.addr #= alignedAddress
      if (busConfig.useId) ar.id #= 0
      if (busConfig.useRegion) ar.region #= 0
      if (busConfig.useLen) ar.len #= 0 // single transfer
      if (busConfig.useSize) ar.size #= 3 // 8 bytes
      if (busConfig.useBurst) ar.burst #= 0 // fixed
      if (busConfig.useLock) ar.lock.randomize()
      if (busConfig.useCache) ar.cache.randomize()
      if (busConfig.useQos) ar.qos.randomize()
      if (busConfig.arUserWidth >= 0) ar.user.randomize()
      if (busConfig.useProt) ar.prot.randomize()
    }

    clockDomain.waitSamplingWhere(!readPending)

    (readValue >> (byteOffset * 8)) & ((BigInt(1) << (8 * numBytes)) - 1)
  }

  def writeBytes(address: BigInt, data: BigInt, numBytes: Int) = {
    assert(numBytes == 1 || numBytes == 2 || numBytes == 4 || numBytes == 8)
    assert(address % numBytes == 0, "address is not aligned")
    assert(data >= 0 && data < (BigInt(1) << (numBytes * 8)))

    val byteOffset = (address & (busConfig.bytePerWord - 1)).toInt
    val alignedAddress = address - byteOffset

    awQueue.enqueue { () =>
      aw.addr #= alignedAddress
      if (busConfig.useId) aw.id #= 0
      if (busConfig.useRegion) aw.region #= 0
      if (busConfig.useLen) aw.len #= 0 // single transfer
      if (busConfig.useSize) aw.size #= 3 // 8 bytes
      if (busConfig.useBurst) aw.burst #= 0 // fixed
      if (busConfig.useLock) aw.lock.randomize()
      if (busConfig.useCache) aw.cache.randomize()
      if (busConfig.useQos) aw.qos.randomize()
      if (busConfig.awUserWidth >= 0) aw.user.randomize()
      if (busConfig.useProt) aw.prot.randomize()
    }

    val strb = (((BigInt(1) << numBytes) - 1) << byteOffset) & ((BigInt(1) << busConfig.bytePerWord) - 1)
    wQueue.enqueue { () =>
      w.data #= data << (byteOffset * 8)
      if (busConfig.useStrb) w.strb #= strb
      if (busConfig.useWUser) w.user.randomize()
      if (busConfig.useLast) w.last #= true
    }

    bQueue.enqueue { () =>
      if (busConfig.useResp) assert(b.resp.toInt == 0)
    }

    clockDomain.waitSamplingWhere(!writePending)
  }

}

// sbt 'testOnly *Axi4TypedDriverTest'
class Axi4TypedDriverTest extends AnyFunSuite {

  case class MockMemory() extends Component {
    val io = new Bundle {
      val s0 = slave(
        Axi4(
          Axi4Config(
            addressWidth = log2Up(16),
            dataWidth = 64,
            useId = false
          )
        )
      )
    }
    val factory = Axi4SlaveFactory(io.s0)
    factory.createWriteAndReadMultiWord(
      UInt(128 bits),
      address = 0,
      documentation = "test"
    ) init (0)
  }

  test("logic validity") {

    Config.sim
      .compile(MockMemory())
      .doSim("logic validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)

        val driver = Axi4TypedDriver(dut.io.s0, dut.clockDomain)

        assert(driver.read_64(0) == 0)
        assert(driver.read_64(8) == 0)

        driver.write_64(0, BigInt("1234567887654321", 16))
        driver.write_64(8, BigInt("2345678998765432", 16))

        assert(driver.read_64(0) == BigInt("1234567887654321", 16))
        assert(driver.read_64(8) == BigInt("2345678998765432", 16))

        assert(driver.read_32(0) == BigInt("87654321", 16))
        assert(driver.read_32(4) == BigInt("12345678", 16))

        assert(driver.read_16(0) == BigInt("4321", 16))
        assert(driver.read_16(2) == BigInt("8765", 16))
        assert(driver.read_16(4) == BigInt("5678", 16))
        assert(driver.read_16(6) == BigInt("1234", 16))

        assert(driver.read_8(0) == BigInt("21", 16))
        assert(driver.read_8(1) == BigInt("43", 16))
        assert(driver.read_8(2) == BigInt("65", 16))
        assert(driver.read_8(3) == BigInt("87", 16))
        assert(driver.read_8(4) == BigInt("78", 16))
        assert(driver.read_8(5) == BigInt("56", 16))
        assert(driver.read_8(6) == BigInt("34", 16))
        assert(driver.read_8(7) == BigInt("12", 16))

        driver.write_32(4, BigInt("ABCDDCBA", 16))
        assert(driver.read_64(0) == BigInt("ABCDDCBA87654321", 16))

        driver.write_64(0, BigInt("1234567887654321", 16))
        driver.write_16(6, BigInt("ABCD", 16))
        assert(driver.read_64(0) == BigInt("ABCD567887654321", 16))

        driver.write_64(0, BigInt("1234567887654321", 16))
        driver.write_8(7, BigInt("AB", 16))
        assert(driver.read_64(0) == BigInt("AB34567887654321", 16))

      }

  }

}
