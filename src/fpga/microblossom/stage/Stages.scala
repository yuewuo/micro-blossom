package microblossom.stage

import microblossom._
import microblossom.types._
import spinal.core._
import spinal.lib._
import scala.collection.mutable.Map

case class Stage(
    val setter: Bundle,
    val getter: Bundle,
    var isRegisterInjected: Boolean = false
)

case class StageOutputs[
    Offload <: Bundle,
    Offload2 <: Bundle,
    Offload3 <: Bundle,
    Offload4 <: Bundle
](
    val offload: () => Offload = () => new Bundle {},
    val offload2: () => Offload2 = () => new Bundle {},
    val offload3: () => Offload3 = () => new Bundle {},
    val offload4: () => Offload4 = () => new Bundle {}
) extends Bundle {
  val offloadGet = offload()
  val offloadGet2 = offload2()
  val offloadGet3 = offload3()
  val offloadGet4 = offload4()
}

case class Stages[Offload <: Bundle, Offload2 <: Bundle, Offload3 <: Bundle, Offload4 <: Bundle](
    val offload: () => Offload = () => new Bundle {},
    val offload2: () => Offload2 = () => new Bundle {},
    val offload3: () => Offload3 = () => new Bundle {},
    val offload4: () => Offload4 = () => new Bundle {}
) {
  private val namedStages = Map[String, Stage]()
  private def addNamedStage(name: String, setter: Bundle, getter: Bundle) = {
    namedStages += (name -> Stage(setter, getter))
  }

  /** at register at a specific stage */
  def injectRegisterAt(name: String) = {
    val stage = namedStages.get(name).get
    require(!stage.isRegisterInjected, "already injected")
    stage.getter := RegNext(stage.setter)
    stage.isRegisterInjected = true
  }

  /** must be called exactly ONCE, to connect all the setter and getter that has not been registered */
  def finish() = {
    for ((name, stage) <- namedStages) {
      if (!stage.isRegisterInjected) {
        stage.getter := stage.setter
      }
    }
  }

  val offloadSet = offload()
  val offloadGet = offload()
  addNamedStage("offload", offloadSet, offloadGet)

  val offloadSet2 = offload2()
  val offloadGet2 = offload2()
  addNamedStage("offload2", offloadSet2, offloadGet2)

  val offloadSet3 = offload3()
  val offloadGet3 = offload3()
  addNamedStage("offload3", offloadSet3, offloadGet3)

  val offloadSet4 = offload4()
  val offloadGet4 = offload4()
  addNamedStage("offload4", offloadSet4, offloadGet4)

  def getStageOutput(): StageOutputs[Offload, Offload2, Offload3, Offload4] = {
    StageOutputs(offload, offload2, offload3, offload4)
  }

  def connectStageOutput(stageOutput: StageOutputs[Offload, Offload2, Offload3, Offload4]) = {
    stageOutput.offloadGet := offloadGet
    stageOutput.offloadGet2 := offloadGet2
    stageOutput.offloadGet3 := offloadGet3
    stageOutput.offloadGet4 := offloadGet4
  }

}
