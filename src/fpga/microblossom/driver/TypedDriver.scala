package microblossom.driver

trait TypedDriver {
  def reset(): Unit
  def readBytes(address: BigInt, numBytes: Int): BigInt
  def writeBytes(address: BigInt, data: BigInt, numBytes: Int): Unit

  def read_64(address: BigInt): BigInt = readBytes(address, 8)
  def read_32(address: BigInt): BigInt = readBytes(address, 4)
  def read_16(address: BigInt): BigInt = readBytes(address, 2)
  def read_8(address: BigInt): BigInt = readBytes(address, 1)
  def write_64(address: BigInt, data: BigInt) = writeBytes(address, data, 8)
  def write_32(address: BigInt, data: BigInt) = writeBytes(address, data, 4)
  def write_16(address: BigInt, data: BigInt) = writeBytes(address, data, 2)
  def write_8(address: BigInt, data: BigInt) = writeBytes(address, data, 1)
}
