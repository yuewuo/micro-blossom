package microblossom.util

object Util {

  def log2(x: Double): Double = {
    return math.log(x) / math.log(2.0)
  }

  def bitsToHold(max_value: Long): Int = {
    return log2(max_value.toDouble + 1).ceil.toInt
  }

}
