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
    Offload4 <: Bundle,
    Execute <: Bundle,
    Execute2 <: Bundle,
    Execute3 <: Bundle,
    Update <: Bundle,
    Update2 <: Bundle,
    Update3 <: Bundle
](
    val offload: () => Offload = () => new Bundle {},
    val offload2: () => Offload2 = () => new Bundle {},
    val offload3: () => Offload3 = () => new Bundle {},
    val offload4: () => Offload4 = () => new Bundle {},
    val execute: () => Execute = () => new Bundle {},
    val execute2: () => Execute2 = () => new Bundle {},
    val execute3: () => Execute3 = () => new Bundle {},
    val update: () => Update = () => new Bundle {},
    val update2: () => Update2 = () => new Bundle {},
    val update3: () => Update3 = () => new Bundle {}
) extends Bundle {
  val offloadGet = offload()
  val offloadGet2 = offload2()
  val offloadGet3 = offload3()
  val offloadGet4 = offload4()
  val executeGet = execute()
  val executeGet2 = execute2()
  val executeGet3 = execute3()
  val updateGet = update()
  val updateGet2 = update2()
  val updateGet3 = update3()
}

case class Stages[
    Offload <: Bundle,
    Offload2 <: Bundle,
    Offload3 <: Bundle,
    Offload4 <: Bundle,
    Execute <: Bundle,
    Execute2 <: Bundle,
    Execute3 <: Bundle,
    Update <: Bundle,
    Update2 <: Bundle,
    Update3 <: Bundle
](
    val offload: () => Offload = () => new Bundle {},
    val offload2: () => Offload2 = () => new Bundle {},
    val offload3: () => Offload3 = () => new Bundle {},
    val offload4: () => Offload4 = () => new Bundle {},
    val execute: () => Execute = () => new Bundle {},
    val execute2: () => Execute2 = () => new Bundle {},
    val execute3: () => Execute3 = () => new Bundle {},
    val update: () => Update = () => new Bundle {},
    val update2: () => Update2 = () => new Bundle {},
    val update3: () => Update3 = () => new Bundle {}
) extends Bundle {
  private val namedStages = Map[String, Stage]()
  def stageNames = namedStages.keys
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

  val executeSet = execute()
  val executeGet = execute()
  addNamedStage("execute", executeSet, executeGet)

  val executeSet2 = execute2()
  val executeGet2 = execute2()
  addNamedStage("execute2", executeSet2, executeGet2)

  val executeSet3 = execute3()
  val executeGet3 = execute3()
  addNamedStage("execute3", executeSet3, executeGet3)

  val updateSet = update()
  val updateGet = update()
  addNamedStage("update", updateSet, updateGet)

  val updateSet2 = update2()
  val updateGet2 = update2()
  addNamedStage("update2", updateSet2, updateGet2)

  val updateSet3 = update3()
  val updateGet3 = update3()
  addNamedStage("update3", updateSet3, updateGet3)

  def getStageOutput()
      : StageOutputs[Offload, Offload2, Offload3, Offload4, Execute, Execute2, Execute3, Update, Update2, Update3] = {
    StageOutputs(offload, offload2, offload3, offload4, execute, execute2, execute3, update, update2, update3)
  }

  def connectStageOutput(
      stageOutput: StageOutputs[
        Offload,
        Offload2,
        Offload3,
        Offload4,
        Execute,
        Execute2,
        Execute3,
        Update,
        Update2,
        Update3
      ]
  ) = {
    stageOutput.offloadGet := offloadGet
    stageOutput.offloadGet2 := offloadGet2
    stageOutput.offloadGet3 := offloadGet3
    stageOutput.offloadGet4 := offloadGet4
    stageOutput.executeGet := executeGet
    stageOutput.executeGet2 := executeGet2
    stageOutput.executeGet3 := executeGet3
    stageOutput.updateGet := updateGet
    stageOutput.updateGet2 := updateGet2
    stageOutput.updateGet3 := updateGet3
  }

}
