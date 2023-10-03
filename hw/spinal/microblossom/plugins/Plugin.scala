package microblossom.plugins

import spinal.core._

trait Plugin extends Nameable {
  setName(this.getClass.getSimpleName.replace("$", ""))

  def setup(): Unit = {}

  def build(): Unit
}
