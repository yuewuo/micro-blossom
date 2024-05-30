package microblossom

import spinal.core._
import spinal.lib._
import spinal.lib.bus.amba4.axi._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.wishbone._
import microblossom._

trait MicroBlossomBusType {
  def generate(config: DualConfig, clockDivideBy: Double = 2, baseAddress: BigInt = 0): Component
}

object MicroBlossomBusType {
  val busTypes: Map[String, MicroBlossomBusType] = Map(
    "Axi4" -> MicroBlossomAxi4,
    "AxiLite4" -> MicroBlossomAxiLite4,
    "AxiLite4Bus32" -> MicroBlossomAxiLite4Bus32,
    "WishboneBus32" -> MicroBlossomWishboneBus32
  )
  def options = busTypes.keys
  def generateByName(
      busTypeName: String,
      config: DualConfig,
      clockDivideBy: Double = 2,
      baseAddress: BigInt = 0
  ): Component = {
    busTypes.get(busTypeName) match {
      case Some(busType) => busType.generate(config = config, clockDivideBy = clockDivideBy, baseAddress = baseAddress)
      case None          => throw new Exception(s"bus type $busTypeName is not recognized")
    }
  }
}

object MicroBlossomAxi4 extends MicroBlossomBusType {
  def generate(config: DualConfig, clockDivideBy: Double = 2, baseAddress: BigInt = 0): Component =
    apply(config = config, clockDivideBy, baseAddress = baseAddress)
  def renamedAxi4(config: Axi4Config) = {
    val axi4 = Axi4(config)
    Axi4SpecRenamer(axi4)
    axi4
  }
  def apply(
      config: DualConfig,
      clockDivideBy: Double = 2,
      baseAddress: BigInt = 0,
      axi4Config: Axi4Config = VersalAxi4Config(addressWidth = log2Up(8 MiB))
  ) = {
    MicroBlossomBus(
      config,
      clockDivideBy,
      baseAddress,
      () => renamedAxi4(axi4Config),
      (x: Axi4) => Axi4SlaveFactory(x)
    )
  }
}

object MicroBlossomAxiLite4 extends MicroBlossomBusType {
  def generate(config: DualConfig, clockDivideBy: Double = 2, baseAddress: BigInt = 0): Component =
    apply(config = config, clockDivideBy, baseAddress = baseAddress)
  def renamedAxiLite4(config: AxiLite4Config) = {
    val axiLite4 = AxiLite4(config)
    AxiLite4SpecRenamer(axiLite4)
    axiLite4
  }
  def apply(
      config: DualConfig,
      clockDivideBy: Double = 2,
      baseAddress: BigInt = 0,
      axiLite4Config: AxiLite4Config = AxiLite4Config(addressWidth = log2Up(8 MiB), dataWidth = 64)
  ) = {
    MicroBlossomBus[AxiLite4, AxiLite4SlaveFactory](
      config,
      clockDivideBy,
      baseAddress,
      () => renamedAxiLite4(axiLite4Config),
      (x: AxiLite4) => {
        AxiLite4SlaveFactory(x)
      }
    )
  }
}

object MicroBlossomAxiLite4Bus32 extends MicroBlossomBusType {
  def generate(config: DualConfig, clockDivideBy: Double = 2, baseAddress: BigInt = 0): Component =
    apply(config = config, clockDivideBy, baseAddress = baseAddress)
  def apply(
      config: DualConfig,
      clockDivideBy: Double = 2,
      baseAddress: BigInt = 0,
      axiLite4Config: AxiLite4Config = AxiLite4Config(addressWidth = log2Up(8 MiB), dataWidth = 32),
      addressWidth: Int = log2Up(8 MiB)
  ) = {
    MicroBlossomBus[AxiLite4, AxiLite4SlaveFactory](
      config,
      clockDivideBy,
      baseAddress,
      () => MicroBlossomAxiLite4.renamedAxiLite4(axiLite4Config),
      (x: AxiLite4) => AxiLite4SlaveFactory(x)
    )
  }
}

// efabless uses 32 bits Wishbone interface, which is a lot simpler than AXI4
// https://caravel-harness.readthedocs.io/en/latest/
// https://caravel-mgmt-soc-litex.readthedocs.io/en/latest/
object MicroBlossomWishboneBus32 extends MicroBlossomBusType {
  def generate(config: DualConfig, clockDivideBy: Double = 2, baseAddress: BigInt = 0): Component =
    apply(config = config, clockDivideBy, baseAddress = baseAddress)
  def apply(
      config: DualConfig,
      clockDivideBy: Double = 2,
      baseAddress: BigInt = 0,
      wishboneConfig: WishboneConfig = WishboneConfig(addressWidth = log2Up(8 MiB), dataWidth = 32),
      addressWidth: Int = log2Up(8 MiB)
  ) = {
    MicroBlossomBus(
      config,
      clockDivideBy,
      baseAddress,
      () => Wishbone(wishboneConfig),
      (x: Wishbone) => WishboneSlaveFactory(x)
    )
  }
}
