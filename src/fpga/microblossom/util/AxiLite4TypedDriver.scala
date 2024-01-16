package microblossom.util

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.amba4.axilite.sim._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import org.scalatest.funsuite.AnyFunSuite

case class AxiLite4TypedDriver(axi: AxiLite4, clockDomain: ClockDomain) {
  val driver = AxiLite4Driver(axi, clockDomain)

  val dataWidth = axi.config.dataWidth
  require(dataWidth == 64 || dataWidth == 32, "only 64 bits or 32 bits bus is supported")
  val is64bus = dataWidth == 64

  def read_bytes(address: BigInt, numBytes: Int): BigInt = {
    assert(numBytes == 1 || numBytes == 2 || numBytes == 4 || numBytes == 8)
    assert(address % numBytes == 0, "address is not aligned")
    if (!is64bus && numBytes == 8) {
      (driver.read(address + 4) << 32) | driver.read(address)
    } else {
      val valueMask = ((BigInt(1) << (numBytes * 8)) - 1)
      val shift = log2Up(dataWidth / 8)
      var value = driver.read((address >> shift) << shift) // force align
      value = value >> ((address % (dataWidth / 8)) * 8).intValue()
      value & valueMask
    }
  }

  def read_64(address: BigInt): BigInt = read_bytes(address, 8)
  def read_32(address: BigInt): BigInt = read_bytes(address, 4)
  def read_16(address: BigInt): BigInt = read_bytes(address, 2)
  def read_8(address: BigInt): BigInt = read_bytes(address, 1)

  def write_bytes(address: BigInt, data: BigInt, numBytes: Int) = {
    assert(numBytes == 1 || numBytes == 2 || numBytes == 4 || numBytes == 8)
    assert(address % numBytes == 0, "address is not aligned")
    assert(data >= 0 && data < (BigInt(1) << (numBytes * 8)))
    if (!is64bus && numBytes == 8) {
      val valueMask = ((BigInt(1) << dataWidth) - 1)
      driver.write(address, data & valueMask)
      driver.write(address + 4, (data >> 32) & valueMask)
    } else {
      // no need to construct strb for partial write because AXI4-Lite "can choose to assume all bytes are valid"
      val shift = log2Up(dataWidth / 8)
      driver.write((address >> shift) << shift, data << ((address % (dataWidth / 8)) * 8).intValue())
    }
  }

  def write_64(address: BigInt, data: BigInt) = write_bytes(address, data, 8)
  def write_32(address: BigInt, data: BigInt) = write_bytes(address, data, 4)
  def write_16(address: BigInt, data: BigInt) = write_bytes(address, data, 2)
  def write_8(address: BigInt, data: BigInt) = write_bytes(address, data, 1)

}

// sbt 'testOnly *AxiLite4TypedDriverTest'
class AxiLite4TypedDriverTest extends AnyFunSuite {

  case class MockMemory(is64bus: Boolean = true) extends Component {
    val io = new Bundle {
      val s0 = slave(
        AxiLite4(
          AxiLite4Config(
            addressWidth = log2Up(16),
            dataWidth = if (is64bus) { 64 }
            else { 32 }
          )
        )
      )
    }
    val factory = AxiLite4SlaveFactory(io.s0)
    factory.createWriteAndReadMultiWord(
      UInt(128 bits),
      address = 0,
      documentation = "test"
    ) init (0)
  }

  test("logic validity") {

    for (is64bus <- List(true, false)) {

      Config.sim
        .compile(MockMemory(is64bus))
        .doSim("logic validity") { dut =>
          dut.clockDomain.forkStimulus(period = 10)

          val driver = AxiLite4TypedDriver(dut.io.s0, dut.clockDomain)

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
          if (is64bus) {
            if (driver.read_64(0) != BigInt("ABCDDCBA87654321", 16)) {
              println("[Warning] bus does not follow strb signals (which is acceptable according to AXI4-Lite spec)")
              assert(driver.read_64(0) == BigInt("ABCDDCBA00000000", 16))
            }
          } else {
            assert(driver.read_64(0) == BigInt("ABCDDCBA87654321", 16))
          }

          driver.write_64(0, BigInt("1234567887654321", 16))
          driver.write_16(6, BigInt("ABCD", 16))
          if (is64bus) {
            if (driver.read_64(0) != BigInt("ABCD567887654321", 16)) {
              println("[Warning] bus does not follow strb signals (which is acceptable according to AXI4-Lite spec)")
              assert(driver.read_64(0) == BigInt("ABCD000000000000", 16))
            }
          } else {
            if (driver.read_64(0) != BigInt("ABCD567887654321", 16)) {
              println("[Warning] bus does not follow strb signals (which is acceptable according to AXI4-Lite spec)")
              assert(driver.read_64(0) == BigInt("ABCD000087654321", 16))
            }
          }

          driver.write_64(0, BigInt("1234567887654321", 16))
          driver.write_8(7, BigInt("AB", 16))
          if (is64bus) {
            if (driver.read_64(0) != BigInt("AB34567887654321", 16)) {
              println("[Warning] bus does not follow strb signals (which is acceptable according to AXI4-Lite spec)")
              assert(driver.read_64(0) == BigInt("AB00000000000000", 16))
            }
          } else {
            if (driver.read_64(0) != BigInt("AB34567887654321", 16)) {
              println("[Warning] bus does not follow strb signals (which is acceptable according to AXI4-Lite spec)")
              assert(driver.read_64(0) == BigInt("AB00000087654321", 16))
            }
          }

        }

    }

  }

}
