package microblossom.util
/*
 * # OneMem
 *
 * The default OneMem has strange behavior when wordCount is 1 (address is 0 bits).
 * This wrapper class handles this difference.
 *
 */

import spinal.core._
import spinal.lib._

object OneMem {
  def apply[T <: Data](wordType: HardType[T], wordCount: Int) = new OneMem(wordType, wordCount)
  def apply[T <: Data](wordType: HardType[T], wordCount: BigInt) = {
    assert(wordCount <= Integer.MAX_VALUE)
    new OneMem(wordType, wordCount.toInt)
  }
}

class OneMem[T <: Data](val wordType: HardType[T], val wordCount: Int, val assertDualPort: Boolean = true)
    extends Bundle {
  var mem: Mem[T] = null
  var register: T = Reg(cloneOf(wordType)).allowPruning()
  var portCount = 0

  if (wordCount != 1) {
    mem = Mem(wordType, wordCount)
  }

  def addressWidth = log2Up(wordCount)

  def init(initialContent: Seq[T]): this.type = {
    assert(initialContent.length == wordCount)
    if (wordCount == 1) {
      register.init(initialContent(0))
    } else {
      mem.init(initialContent)
    }
    this
  }

  def portCreated() = {
    portCount += 1
    if (assertDualPort) {
      assert(portCount <= 2, "creating more than 2 ports, may not be synthesizable")
    }
  }

  def readSync(
      address: UInt,
      enable: Bool = null,
      readUnderWrite: ReadUnderWritePolicy = dontCare,
      clockCrossing: Boolean = false
  ): T = {
    portCreated()
    if (wordCount == 1) {
      assert(address.getBitsWidth == 0)
      assert(readUnderWrite == dontCare)
      assert(clockCrossing == false)
      if (enable == null) {
        RegNext(register)
      } else {
        RegNextWhen(register, enable)
      }
    } else {
      mem.readSync(address, enable, readUnderWrite, clockCrossing)
    }
  }

  def write(address: UInt, data: T, enable: Bool = null, mask: Bits = null): Unit = {
    portCreated()
    if (wordCount == 1) {
      assert(address.getBitsWidth == 0)
      assert(mask == null)
      if (enable == null) {
        register := data
      } else {
        when(enable) {
          register := data
        }
      }
    } else {
      mem.write(address, data, enable, mask)
    }
  }

  def readWriteSync(
      address: UInt,
      data: T,
      enable: Bool,
      write: Bool,
      mask: Bits = null,
      readUnderWrite: ReadUnderWritePolicy = dontCare,
      clockCrossing: Boolean = false,
      duringWrite: DuringWritePolicy = dontCare
  ): T = {
    portCreated()
    if (wordCount == 1) {
      assert(address.getBitsWidth == 0)
      assert(mask == null)
      assert(clockCrossing == false)
      assert(readUnderWrite == dontCare)
      assert(duringWrite == dontCare)
      when(enable && write) {
        register := data
      }
      RegNextWhen(register, enable)
    } else {
      mem.readWriteSync(address, data, enable, write, mask, readUnderWrite, clockCrossing, duringWrite)
    }
  }

  def constructReadWriteSyncChannel(): ReadWriteSyncChannel[T] = {
    val channel = new ReadWriteSyncChannel(wordType, addressWidth)
    channel.data := readWriteSync(channel.address, channel.writeData, channel.enable, channel.write)
    channel
  }

  def constructReadSyncChannel(): ReadSyncChannel[T] = {
    val channel = new ReadSyncChannel(wordType, addressWidth)
    channel.data := readSync(channel.address, channel.enable)
    channel
  }

}

class ReadWriteSyncChannel[T <: Data](val wordType: HardType[T], val addressWidth: Int) extends Bundle {
  val address = UInt(addressWidth bits)
  val writeData = cloneOf(wordType)
  val enable = Bool
  val write = Bool
  val data = cloneOf(wordType)

  enable := False
  write := False
  address.assignDontCare()
  writeData.assignDontCare()

  def readNext(address: UInt) = {
    enable := True
    write := False
    this.address := address
  }

  def writeNext(address: UInt, data: T) = {
    enable := True
    write := True
    this.address := address
    writeData := data
  }
}

class ReadSyncChannel[T <: Data](val wordType: HardType[T], val addressWidth: Int) extends Bundle {
  val address = UInt(addressWidth bits)
  val enable = Bool
  val data = cloneOf(wordType)

  enable := False
  address.assignDontCare()

  def readNext(address: UInt) = {
    enable := True
    this.address := address
  }
}
