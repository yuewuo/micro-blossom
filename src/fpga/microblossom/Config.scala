package microblossom

import spinal.core._
import spinal.core.sim._
import java.io.File
import spinal.lib.bus.amba4.axi._

object Config {
  def spinal(targetDirectory: String = "gen") = SpinalConfig(
    targetDirectory = targetDirectory,
    defaultConfigForClockDomains = ClockDomainConfig(
      resetActiveLevel = HIGH
    ),
    onlyStdLogicVectorAtTopLevelIo = true
  )

  def sim = SimConfig.withConfig(spinal()).withFstWave

  def argFolderPath(folderPath: String) = {
    val folder = new File(folderPath)
    if (!folder.exists) {
      Console.err.println(s"please ensure folder exists: $folderPath")
      sys.exit(1)
    }
    if (!folder.isDirectory) {
      Console.err.println(s"folder path is not a directory: $folderPath")
      sys.exit(1)
    }
    spinal(folderPath)
  }
}

object VersalAxi4Config {
  def apply() = {
    Axi4Config(
      addressWidth = 44,
      dataWidth = 64,
      idWidth = 16,
      arUserWidth = 16,
      awUserWidth = 16,
      useRegion = false
    )
  }
}

object MinimalAxi4Config {
  def apply(addressWidth: Int = 12) = {
    Axi4Config(
      addressWidth = addressWidth,
      dataWidth = 64,
      useId = false,
      useRegion = false,
      useBurst = false,
      useLock = false,
      useCache = false,
      useSize = false,
      useQos = false
    )
  }
}
