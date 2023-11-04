package microblossom

import spinal.core._
import spinal.core.sim._

object Config {
  def spinal(targetDirectory: String = "gen") = SpinalConfig(
    targetDirectory = targetDirectory,
    defaultConfigForClockDomains = ClockDomainConfig(
      resetActiveLevel = HIGH
    ),
    onlyStdLogicVectorAtTopLevelIo = true
  )

  def sim = SimConfig.withConfig(spinal()).withFstWave
}
