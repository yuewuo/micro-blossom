package microblossom

/*
 * # Micro Blossom Accelerator
 *
 * This module provides unified access to the Distributed Dual module with AXI4 interface.
 *
 *
 * Note:
 *     1. When you issue instructions, by default we set maximumGrowth to 0 to avoid any spontaneous growth
 *        between the execution of your instructions. However, if you explicitly issue a `FindObstacle` instruction,
 *        we will use the `maximumGrowth` value you set last time. This is useful when you have context switching
 *        because you can issue `FindObstacle` and then the readout value will be cached so that it will be fast the
 *        the next time you actually read the obstacle.
 *     2. Current implementation only supports one obstacle channel. The address space allows for multiple obstacles
 *        but implementation is missing.
 *     3. To maximum the throughput, we allow obstacle pre-fetching by issuing a FindObstacle explicitly. If the
 *        `maximumGrowth` value is not changed from last time, the result is fed back immediately.
 *
 */

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._
import spinal.core._
import spinal.lib._
import spinal.lib.fsm._
import spinal.lib.bus.amba4.axi._
import spinal.lib.bus.amba4.axilite._
import spinal.lib.bus.amba4.axilite.sim._
import spinal.lib.bus.wishbone._
import spinal.lib.bus.regif._
import spinal.lib.bus.misc._
import spinal.core.sim._
import microblossom._
import microblossom.util._
import microblossom.driver._
import microblossom.types._
import microblossom.modules._
import microblossom.stage._
import org.scalatest.funsuite.AnyFunSuite
import org.rogach.scallop._

// max d=31 (31^3 < 32768), for 0.1% physical error rate we have 18 reported obstacles on average
// since there is no need to save memory space, we just allocate whatever convenient; for now we assume 8MB
// 1. 128KB control block at [0, 0x2_0000]
//    0: (RO) 64 bits timer counter
//    8: (RO) 32 bits version register
//    12: (RO) 32 bits context depth
//    16: (RO) 8 bits number of conflict channels (no more than 6 is supported)
//    17: (RO) 8 bits dualConfig.vertexBits
//    18: (RO) 8 bits dualConfig.weightBits
//    19: (RO) 8 bits dualConfig.instructionBufferDepth
//    20: (RO) 16 bits configuration bits: (..., is64bus:5, supportContextSwitching:4
//                    hardCodeWeights:3, supportLayerFusion:2, supportOffloading:1, supportAddDefectVertex:0)
//
//    24: (RW) 32 bits instruction counter
//    32: (RW) 32 bits readout counter
//    40: (RW) 32 bits transaction counter
//    48: (RW) 32 bits error counter
//  - (64 bits only) the following 4KB section is designed to allow burst writes (e.g. use xsdb "mwr -bin -file" command)
//    0x1000: (WO) (32 bits instruction, 16 bits context id)
//    0x1008: (WO) (32 bits instruction, 16 bits context id)
//    0x1010: ... repeat for 512: in total 4KB space
//    0x1FFC
//  - (32 bits only) the following 4KB section is designed for 32 bit bus where context id is encoded in the address
//    0x2000: 32 bits instruction for context 0
//    0x2004: 32 bits instruction for context 1
//    0x2008: ... repeat for 1024: in total 4KB space
//    0x2FFC
// 2. 128KB context readouts at [0x2_0000, 0x4_0000), each context takes 128 byte space, assuming no more than 1024 contexts
//    [context 0]
//      0: (R) 64 bits timestamp of receiving the last ``load obstacles'' instruction
//             (reading this address will also waiting for all the transactions of this context to be completed)
//      0: (W:clear) clear the 16 bits accumulated_grown value
//      8: (R) 64 bits timestamp of receiving the last ``growable = infinity'' response
//      16: (RW) 16 bits maximum growth write (offloaded primal), when 0, disable offloaded primal,
//                  write to this field will automatically clear accumulated grown value
//      18: (R) 16 bits growable value (writing to this position has no effect)
//      32: (R) head + conflict (conflict_fields: 96 bits, conflict_valid: u8, growable: u8, accumulated_grown: u16)
//               conflict_fields: (node1, node2, touch1, touch2, vertex1, vertex2, each 16 bits)
//               here we use u8 sized growable because it
//      (at most 6 concurrent conflict report, large enough)
//      48: next obstacle, the head remains the same
//        ...
//    [context 1]
//      128: ...
//

case class InstructionTag(config: DualConfig) extends Bundle {
  val instructionId = UInt(config.instructionBufferBits bits)
  val isFindObstacle = Bool
  val accumulatedGrown = UInt(16 bits)
}

case class MicroBlossomBus[T <: IMasterSlave, F <: BusSlaveFactoryDelayed](
    config: DualConfig,
    clockDivideBy: Double = 2, // the slow clock domain has a frequency of `frequency/clockDivideBy`
    baseAddress: BigInt = 0,
    interfaceBuilder: () => T,
    slaveFactory: (T) => F
) extends Component {
  val io = new Bundle {
    val s0 = slave(interfaceBuilder())
    val slowClk = in Bool ()
  }

  val slowClk = io.slowClk
  slowClk.setName("slow_clk")

  val rawFactory = slaveFactory(io.s0)
  val factory = rawFactory.withOffset(baseAddress)

  require(config.conflictChannels == 1, "not implemented: multiple conflict channels")
  require(clockDivideBy >= 1, "the bus must run at higher frequency, otherwise data loss might occur")
  require(factory.busDataWidth == 64 || factory.busDataWidth == 32, "only 64 bits or 32 bits bus is supported")
  val is64bus = factory.busDataWidth == 64

  // 0: (RO) 64 bits timer counter
  val counter = new Area {
    val value = Reg(UInt(64 bits)) init 0
    value := value + 1
    factory.readMultiWord(value, 0, documentation = "64 bits timer")
  }

  // 8: (RO) 32 bits version register
  // 12: (RO) 32 bits context depth
  // 16: (RO) 8 bits number of conflict channels (we're not using 100+ conflict channels...)
  val configurationBits = B(0, 16 bits)
  configurationBits(0) := Bool(config.supportAddDefectVertex)
  configurationBits(1) := Bool(config.supportOffloading)
  configurationBits(2) := Bool(config.supportLayerFusion)
  configurationBits(3) := Bool(config.hardCodeWeights)
  configurationBits(4) := Bool(config.supportContextSwitching)
  configurationBits(5) := Bool(is64bus)
  val hardwareInfo = new Area {
    factory.readMultiWord(
      U(config.contextDepth, 32 bits) ## U(DualConfig.version, 32 bits),
      address = 8,
      documentation = "micro-blossom version and context depth"
    )
    factory.readMultiWord(
      configurationBits ##
        U(config.instructionBufferDepth, 8 bits) ## U(config.weightBits, 8 bits) ##
        U(config.vertexBits, 8 bits) ## U(config.conflictChannels, 8 bits),
      address = 16,
      documentation = "the number of conflict channels"
    )
    val instructionCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 24,
        documentation = "instruction counter"
      ) init (0)
    val readoutCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 32,
        documentation = "readout counter"
      ) init (0)
    val transactionCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 40,
        documentation = "number of AXI4 transactions"
      ) init (0)
    val errorCounter =
      factory.createWriteAndReadMultiWord(
        UInt(32 bits),
        address = 48,
        documentation = "error counter"
      ) init (0)
  }
  val hasError = Bool
  hasError := False
  when(hasError) {
    hardwareInfo.errorCounter := hardwareInfo.errorCounter + 1
  }

  val slowClockDomain = ClockDomain(
    clock = slowClk,
    reset = ClockDomain.current.readResetWire,
    config = ClockDomainConfig(
      clockEdge = RISING,
      resetKind = SYNC,
      resetActiveLevel = HIGH
    )
  )

  val ccFifoPush = StreamFifoCC(
    dataType = LooperInput(config, InstructionTag(config)),
    depth = config.instructionBufferDepth,
    pushClock = clockDomain,
    popClock = slowClockDomain
  )
  val ccFifoPop = StreamFifoCC(
    dataType = LooperOutput(config, InstructionTag(config)),
    depth = config.instructionBufferDepth,
    pushClock = slowClockDomain,
    popClock = clockDomain
  )
  val slow = new ClockingArea(slowClockDomain) {
    val microBlossom = MicroBlossomLooper(config, InstructionTag(config))
    microBlossom.io.push << ccFifoPush.io.pop
    microBlossom.io.pop >> ccFifoPop.io.push
  }
  def microBlossom = slow.microBlossom
  ccFifoPush.io.push.valid := False
  ccFifoPush.io.push.payload.assignDontCare()
  ccFifoPop.io.pop.ready := False

  // create the control registers
  val maximumGrowth = OneMem(UInt(16 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 16 bits))
  val accumulatedGrown = OneMem(UInt(16 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 16 bits))
  val maxGrowable = OneMem(ConvergecastMaxGrowable(config.weightBits), config.contextDepth)
  val conflict = OneMem(ConvergecastConflict(config.vertexBits), config.contextDepth)
  val pushId = OneMem(UInt(config.instructionBufferBits bits), config.contextDepth) init
    List.fill(config.contextDepth)(U(0, config.instructionBufferBits bits))
  val popId = OneMem(UInt(config.instructionBufferBits bits), config.contextDepth) init
    List.fill(config.contextDepth)(U(0, config.instructionBufferBits bits))
  val timeLoad = new Area {
    val lower = OneMem(UInt(32 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 32 bits))
    val upper = OneMem(UInt(32 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 32 bits))
  }
  val timeFinish = new Area {
    val lower = OneMem(UInt(32 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 32 bits))
    val upper = OneMem(UInt(32 bits), config.contextDepth) init List.fill(config.contextDepth)(U(0, 32 bits))
  }
  val isLastFindObstacle = OneMem(Bool, config.contextDepth) init List.fill(config.contextDepth)(False)
  val hasPendingFindObstacle = OneMem(Bool, config.contextDepth) init List.fill(config.contextDepth)(False)

  def getSimDriver(): TypedDriver = {
    if (io.s0.isInstanceOf[AxiLite4]) {
      AxiLite4TypedDriver(io.s0.asInstanceOf[AxiLite4], clockDomain)
    } else if (io.s0.isInstanceOf[Axi4]) {
      Axi4TypedDriver(io.s0.asInstanceOf[Axi4], clockDomain)
    } else {
      throw new Exception("simulator driver not implemented")
    }
  }

  // define memory mappings for control registers
  // write address: factory.writeAddress()
  val instruction = new Area {
    val value = Instruction(DualConfig())
    val contextId = UInt(config.contextBits bits)
    val (mapping, documentation) = if (is64bus) {
      (SizeMapping(base = 4 KiB, size = 4 KiB), "instruction array (64 bits)")
    } else {
      (SizeMapping(base = 8 KiB, size = 4 KiB), "instruction array (32 bits)")
    }
  }
  if (is64bus) {
    factory.nonStopWrite(instruction.value, bitOffset = 0)
    factory.nonStopWrite(instruction.contextId, bitOffset = 32)
  } else {
    factory.nonStopWrite(instruction.value, bitOffset = 0)
    instruction.contextId := factory.writeAddress().resized
  }
  // read address: factory.readAddress()
  val readout = new Area {
    val value = Bits(factory.busDataWidth bits).assignDontCare()
    val writeValue = UInt(factory.busDataWidth bits)
    val contextIdRaw = (factory.readAddress().resize(log2Up(128 KiB)) >> log2Up(128)).resize(10)
    val contextId = contextIdRaw.resize(config.contextBits)
    val subAddress = factory.readAddress().resize(7)
    val obstacleAddress = subAddress.resize(4) // each obstacle entry is 16 bytes = 128 bits
    val writeContextIdRaw = (factory.writeAddress().resize(log2Up(128 KiB)) >> log2Up(128))
    val writeContextId = writeContextIdRaw.resize(config.contextBits bits)
    val writeSubAddress = factory.writeAddress().resize(log2Up(128))
    val mapping = SizeMapping(base = 128 KiB, size = 128 KiB)
    val documentation = "readout array (128 bytes each, 1024 of them at most)"
  }
  // set the values
  factory.readPrimitive(readout.value, readout.mapping, 0, "readouts")
  factory.nonStopWrite(readout.writeValue, bitOffset = 0)

  // define bus behavior when reading or writing the control registers
  val isWriteHalt = Bool.setAll()
  val isWriteAsk = Bool.clearAll()
  val isWriteDo = Bool.clearAll()
  val isReadHalt = Bool.setAll()
  val isReadAsk = Bool.clearAll()
  val isReadDo = Bool.clearAll()
  def onAskWrite() = {
    isWriteAsk := True
    when(isWriteHalt) {
      factory.writeHalt()
    }
  }
  def onDoWrite() = {
    hardwareInfo.transactionCounter := hardwareInfo.transactionCounter + 1
    isWriteDo := True
  }
  def onAskRead() = {
    isReadAsk := True
    when(isReadHalt) {
      factory.readHalt()
    }
  }
  def onDoRead() = {
    hardwareInfo.transactionCounter := hardwareInfo.transactionCounter + 1
    isReadDo := True
  }
  factory.onWritePrimitive(instruction.mapping, haltSensitive = false, instruction.documentation)(onAskWrite)
  factory.onWritePrimitive(instruction.mapping, haltSensitive = true, instruction.documentation)(onDoWrite)
  factory.onWritePrimitive(readout.mapping, haltSensitive = false, readout.documentation)(onAskWrite)
  factory.onWritePrimitive(readout.mapping, haltSensitive = true, readout.documentation)(onDoWrite)
  factory.onReadPrimitive(readout.mapping, haltSensitive = false, readout.documentation)(onAskRead)
  factory.onReadPrimitive(readout.mapping, haltSensitive = true, readout.documentation)(onDoRead)

  // use state machine to handle read and write transactions; read has higher priority because it's blocking operation
  val resizedInstruction = Instruction(config)
  val resizeInstructionHasError = resizedInstruction.resizedFrom(instruction.value)
  // Mem channels that are used by this fsm
  val fsmAccumulatedGrown = accumulatedGrown.constructReadWriteSyncChannel()
  val fsmMaxGrowable = maxGrowable.constructReadSyncChannel()
  val fsmConflict = conflict.constructReadSyncChannel()
  val fsmPushId = pushId.constructReadWriteSyncChannel()
  val fsmPopId = popId.constructReadSyncChannel()
  val fsmMaximumGrowth = maximumGrowth.constructReadWriteSyncChannel()
  val fsmTimeLoad = new Area {
    val lower = timeLoad.lower.constructReadWriteSyncChannel()
    val upper = timeLoad.upper.constructReadWriteSyncChannel()
  }
  val fsmTimeFinish = new Area {
    val lower = timeFinish.lower.constructReadWriteSyncChannel()
    val upper = timeFinish.upper.constructReadWriteSyncChannel()
  }
  val fsmIsLastFindObstacle = isLastFindObstacle.constructReadWriteSyncChannel()
  val fsmHasPendingFindObstacle = hasPendingFindObstacle.constructReadWriteSyncChannel()
  val fsmConflictB16 = ConvergecastConflict(16).resizedFrom(fsmConflict.data)
  val fsmMaxGrowableU8 = ConvergecastMaxGrowable(8).resizedFrom(fsmMaxGrowable.data)
  val fsmMaxGrowableU16 = ConvergecastMaxGrowable(16).resizedFrom(fsmMaxGrowable.data)
  // data that are used to communicate between states
  val dataWaitFiFoPushLooperInput = Reg(LooperInput(config, InstructionTag(config)))
  val fsm = new StateMachine {
    setEncoding(binaryOneHot)

    def readErrorReset() = {
      readout.value.setAll()
      hasError := True
      isReadHalt := False
      goto(stateIdle)
    }

    val stateIdle: State = new State with EntryPoint {
      whenIsActive {
        when(isReadAsk) {
          when(readout.mapping.hit(factory.readAddress())) {
            when(readout.subAddress === U(0)) {
              goto(stateReadLoadTime)
            } elsewhen (readout.subAddress === U(8)) {
              goto(stateReadFinishTime)
            } elsewhen (readout.subAddress === U(16)) {
              goto(stateReadMaximumGrowth)
            } elsewhen (readout.subAddress >= U(32) && readout.subAddress < U(48)) {
              hardwareInfo.readoutCounter := hardwareInfo.readoutCounter + 1
              goto(stateReadObstacle)
            } otherwise {
              if (is64bus) {
                readErrorReset()
              } else {
                when(readout.subAddress === U(4)) {
                  goto(stateReadLoadTimeUpper)
                } elsewhen (readout.subAddress === U(12)) {
                  goto(stateReadFinishTimeUpper)
                } otherwise {
                  readErrorReset()
                }
              }
            }
          } otherwise {
            readErrorReset()
          }
        } elsewhen (isWriteAsk) {
          when(instruction.mapping.hit(factory.writeAddress())) {
            hardwareInfo.instructionCounter := hardwareInfo.instructionCounter + 1
            goto(stateWriteInstruction)
          } elsewhen (readout.mapping.hit(factory.writeAddress())) {
            when(readout.writeSubAddress === U(16)) {
              goto(stateUpdateMaximumGrowth)
            } elsewhen (readout.writeSubAddress === U(0)) {
              fsmAccumulatedGrown.writeNext(readout.writeContextId, U(0))
              isWriteHalt := False
            } otherwise {
              hasError := True
              isWriteHalt := False
            }
          } otherwise {
            hasError := True
            isWriteHalt := False
          }
        }
      }
    }

    val stateUpdateMaximumGrowth: State = new State {
      whenIsNext {
        // check for overflow error
        when(readout.writeValue > fsmMaximumGrowth.data.maxValue) { hasError := True }
        when(readout.writeContextIdRaw > readout.writeContextId.maxValue) { hasError := True }
        // get current value
        fsmMaximumGrowth.readNext(readout.writeContextId)
      }
      whenIsActive {
        val newMaximumGrowth = readout.writeValue.resize(16)
        when(fsmMaximumGrowth.data =/= newMaximumGrowth) {
          // updating maximum growth will invalidate the previous readout value
          fsmIsLastFindObstacle.writeNext(readout.writeContextId, False)
        }
        fsmMaximumGrowth.writeNext(readout.writeContextId, newMaximumGrowth)
        isWriteHalt := False
        goto(stateIdle)
      }
    }

    val stateReadMaximumGrowth: State = new State {
      whenIsNext {
        when(readout.contextIdRaw > readout.contextId.maxValue) { hasError := True }
        fsmMaximumGrowth.readNext(readout.contextId)
        fsmMaxGrowable.readNext(readout.contextId)
      }
      whenIsActive {
        val value = fsmMaxGrowableU16.length ## fsmMaximumGrowth.data
        if (is64bus) { readout.value := U(0, 32 bits) ## value }
        else { readout.value := value }
        isReadHalt := False
        goto(stateIdle)
      }
    }

    // note: whenever entering this state, must not conflict with the fields in watchObstacleFields
    // if conflict occurs, wait for another state before entering this state
    val stateReadObstacle: State = new State {
      whenIsNext {
        fsmPopId.readNext(readout.contextId)
        fsmMaximumGrowth.readNext(readout.contextId)
        fsmAccumulatedGrown.readNext(readout.contextId)
        fsmMaxGrowable.readNext(readout.contextId)
        fsmConflict.readNext(readout.contextId)
        fsmPushId.readNext(readout.contextId)
        fsmIsLastFindObstacle.readNext(readout.contextId)
      }
      whenIsActive {
        when(fsmIsLastFindObstacle.data) {
          when(fsmPushId.data === fsmPopId.data) {
            // data is ready, output and then return to idle
            isReadHalt := False
            goto(stateIdle)
            val value0 = fsmConflictB16.node2 ## fsmConflictB16.node1
            val value4 = fsmConflictB16.touch2 ## fsmConflictB16.touch1
            val value8 = fsmConflictB16.vertex2 ## fsmConflictB16.vertex1
            val value12 = fsmAccumulatedGrown.data ## fsmMaxGrowableU8.length ## fsmConflictB16.valid.asBits(8 bits)
            when(readout.obstacleAddress === U(0)) {
              if (is64bus) { readout.value := value4 ## value0 }
              else { readout.value := value0 }
            } elsewhen (readout.obstacleAddress === U(8)) {
              if (is64bus) { readout.value := value12 ## value8 }
              else { readout.value := value8 }
            } otherwise {
              if (is64bus) { readErrorReset() }
              else {
                when(readout.obstacleAddress === U(4)) {
                  readout.value := value4
                } elsewhen (readout.obstacleAddress === U(12)) {
                  readout.value := value12
                } otherwise {
                  readErrorReset()
                }
              }
            }
          }
          // otherwise just wait until the command (has issued a FindObstacle instruction)
        } otherwise {
          // issue a FindObstacle instruction and then return to this state to wait for the result
          goto(stateReadIssueFindObstacleWaitPending)
        }
      }
    }

    // must entre this state before issuing an obstacle: issuing multiple FindObstacle instruction
    // in the queue causes data races, because the maximumGrowth value is not updated according to the last
    // accumulatedGrown value
    val stateReadIssueFindObstacleWaitPending: State = new State {
      whenIsNext {
        fsmHasPendingFindObstacle.readNext(readout.contextId)
      }
      whenIsActive {
        when(!fsmHasPendingFindObstacle.data) {
          goto(stateReadIssueFindObstacle)
        }
      }
    }

    val stateReadIssueFindObstacle: State = new State {
      whenIsNext {
        fsmPushId.readNext(readout.contextId)
        fsmMaximumGrowth.readNext(readout.contextId)
        fsmAccumulatedGrown.readNext(readout.contextId)
      }
      whenIsActive {
        val instructionId = fsmPushId.data + 1
        // prepare the looper input
        val looperInput = LooperInput(config, InstructionTag(config))
        looperInput.instruction.assignFindObstacle()
        if (config.contextBits > 0) { looperInput.contextId := readout.contextId }
        looperInput.tag.instructionId := instructionId
        looperInput.tag.isFindObstacle := True
        looperInput.tag.accumulatedGrown := fsmAccumulatedGrown.data
        looperInput.maximumGrowth := fsmMaximumGrowth.data - fsmAccumulatedGrown.data
        // prepare the data into the push FIFO
        ccFifoPush.io.push.valid := True
        ccFifoPush.io.push.payload := looperInput
        // increment push ID
        fsmPushId.writeNext(readout.contextId, instructionId)
        fsmIsLastFindObstacle.writeNext(readout.contextId, True)
        fsmHasPendingFindObstacle.writeNext(readout.contextId, True)
        // update state machine
        when(ccFifoPush.io.push.ready) {
          goto(stateReadIssueFindObstaclePause)
        } otherwise {
          dataWaitFiFoPushLooperInput := looperInput
          goto(stateReadIssueFindObstacleWaitFiFoPush)
        }
      }
    }

    // since fsmPushId is already used for writing, we need to wait for a clock cycle before
    // returning to the stateReadObstacle state
    val stateReadIssueFindObstaclePause: State = new State {
      whenIsActive {
        goto(stateReadObstacle)
      }
    }

    val stateReadIssueFindObstacleWaitFiFoPush: State = new State {
      whenIsActive {
        ccFifoPush.io.push.valid := True
        ccFifoPush.io.push.payload := dataWaitFiFoPushLooperInput
        when(ccFifoPush.io.push.ready) {
          goto(stateReadObstacle)
        }
      }
    }

    val stateReadLoadTime: State = new State {
      whenIsNext {
        fsmTimeLoad.upper.readNext(readout.contextId)
        fsmTimeLoad.lower.readNext(readout.contextId)
        fsmPushId.readNext(readout.contextId)
        fsmPopId.readNext(readout.contextId)
      }
      whenIsActive {
        when(fsmPushId.data === fsmPopId.data) {
          if (is64bus) { readout.value := fsmTimeLoad.upper.data.asBits ## fsmTimeLoad.lower.data.asBits }
          else { readout.value := fsmTimeLoad.lower.data.asBits }
          isReadHalt := False
          goto(stateIdle)
        }
      }
    }

    // exist only for 32 bit bus
    var stateReadLoadTimeUpper: State = null
    if (!is64bus) {
      stateReadLoadTimeUpper = new State {
        whenIsNext {
          fsmTimeLoad.upper.readNext(readout.contextId)
        }
        whenIsActive {
          readout.value := fsmTimeLoad.upper.data.asBits
          isReadHalt := False
          goto(stateIdle)
        }
      }
    }

    val stateReadFinishTime: State = new State {
      whenIsNext {
        fsmTimeFinish.upper.readNext(readout.contextId)
        fsmTimeFinish.lower.readNext(readout.contextId)
      }
      whenIsActive {
        if (is64bus) { readout.value := fsmTimeFinish.upper.data.asBits ## fsmTimeFinish.lower.data.asBits }
        else { readout.value := fsmTimeFinish.lower.data.asBits }
        isReadHalt := False
        goto(stateIdle)
      }
    }

    // exist only for 32 bit bus
    var stateReadFinishTimeUpper: State = null
    if (!is64bus) {
      stateReadFinishTimeUpper = new State {
        whenIsNext {
          fsmTimeFinish.upper.readNext(readout.contextId)
        }
        whenIsActive {
          readout.value := fsmTimeFinish.upper.data.asBits
          isReadHalt := False
          goto(stateIdle)
        }
      }
    }

    val stateRead: State = new State {
      whenIsActive {
        readout.value.clearAll()
        isReadHalt := False
        goto(stateIdle)
      }
    }

    val stateWriteInstruction: State = new State {
      onEntry {
        // check what type of the instruction and record timestamp if necessary
        when(instruction.value.isChangingSyndrome) {
          fsmTimeLoad.upper.writeNext(instruction.contextId, counter.value(63 downto 32))
          fsmTimeLoad.lower.writeNext(instruction.contextId, counter.value(31 downto 0))
          // also mark it as not finished
          fsmTimeFinish.upper.writeNext(instruction.contextId, ~U(0, 32 bits))
          fsmTimeFinish.lower.writeNext(instruction.contextId, ~U(0, 32 bits))
        }
        fsmHasPendingFindObstacle.readNext(instruction.contextId)
        fsmMaximumGrowth.readNext(instruction.contextId)
        fsmAccumulatedGrown.readNext(instruction.contextId)
      }
      whenIsActive {
        val isFindObstacle = instruction.value.isFindObstacle
        when(!fsmHasPendingFindObstacle.data || !isFindObstacle) {
          val instructionId = fsmPushId.data + 1
          // prepare the looper input
          val looperInput = LooperInput(config, InstructionTag(config))
          looperInput.instruction := resizedInstruction
          if (config.contextBits > 0) { looperInput.contextId := instruction.contextId }
          looperInput.tag.instructionId := instructionId
          looperInput.tag.isFindObstacle := isFindObstacle
          looperInput.tag.accumulatedGrown := fsmAccumulatedGrown.data
          when(isFindObstacle) {
            looperInput.maximumGrowth := fsmMaximumGrowth.data - fsmAccumulatedGrown.data
          } otherwise {
            looperInput.maximumGrowth.clearAll()
          }
          // prepare the data into the push FIFO
          ccFifoPush.io.push.valid := True
          ccFifoPush.io.push.payload := looperInput
          // check for error
          when(resizeInstructionHasError) { hasError := True }
          // increment push ID
          fsmPushId.writeNext(instruction.contextId, instructionId)
          fsmIsLastFindObstacle.writeNext(instruction.contextId, isFindObstacle)
          when(isFindObstacle) {
            fsmHasPendingFindObstacle.writeNext(instruction.contextId, True)
          }
          // update state machine
          when(ccFifoPush.io.push.ready) {
            isWriteHalt := False
            goto(stateIdle)
          } otherwise {
            dataWaitFiFoPushLooperInput := looperInput
            goto(stateWriteWaitFiFoPush)
          }
        }
      }
    }

    val stateWriteWaitFiFoPush: State = new State {
      whenIsActive {
        ccFifoPush.io.push.valid := True
        ccFifoPush.io.push.payload := dataWaitFiFoPushLooperInput
        when(ccFifoPush.io.push.ready) {
          isWriteHalt := False
          goto(stateIdle)
        }
      }
    }
  }

  // handle the response from the Micro Blossom Looper module
  val rspPopId = popId.constructReadWriteSyncChannel()
  val rspMaxGrowable = maxGrowable.constructReadWriteSyncChannel()
  val rspConflict = conflict.constructReadWriteSyncChannel()
  val rspAccumulatedGrown = accumulatedGrown.constructReadWriteSyncChannel()
  val rspTimeFinish = new Area {
    val upper = timeFinish.upper.constructReadWriteSyncChannel()
    val lower = timeFinish.lower.constructReadWriteSyncChannel()
  }
  val rspHasPendingFindObstacle = hasPendingFindObstacle.constructReadWriteSyncChannel()
  val rsp = new StateMachine {
    setEncoding(binaryOneHot)

    val stateIdle: State = new State with EntryPoint {
      whenIsActive {
        ccFifoPop.io.pop.ready := True
        when(ccFifoPop.io.pop.valid) {
          val output = ccFifoPop.io.pop.payload
          val contextId: UInt = if (config.contextBits > 0) { output.contextId }
          else { UInt(0 bits) }
          // record the last time it sees `maxGrowable = _.maxValue`: marking the finish of decoding
          when(output.maxGrowable.length === output.maxGrowable.length.maxValue) {
            rspTimeFinish.upper.writeNext(contextId, counter.value(63 downto 32))
            rspTimeFinish.lower.writeNext(contextId, counter.value(31 downto 0))
          }
          rspPopId.writeNext(contextId, output.tag.instructionId)
          rspMaxGrowable.writeNext(contextId, output.maxGrowable)
          rspConflict.writeNext(contextId, output.conflict)
          rspAccumulatedGrown.writeNext(contextId, output.grown + output.tag.accumulatedGrown)
          // if the instruction is FindObstacle, unset pending instruction
          when(output.tag.isFindObstacle) {
            rspHasPendingFindObstacle.writeNext(contextId, False)
          }
        }
      }
    }
  }

  def simMakePublicSnapshot() = microBlossom.simMakePublicSnapshot()
  def simSnapshot(abbrev: Boolean = true): Json = microBlossom.simSnapshot(abbrev)
  def simMakePublicPreMatching() = microBlossom.simMakePublicPreMatching()
  def simPreMatchings(): Seq[DataPreMatching] = microBlossom.simPreMatchings()
}

// sbt 'testOnly *MicroBlossomBusTest'
class MicroBlossomBusTest extends AnyFunSuite {

  test("logic_validity") {
    val config = DualConfig(filename = "./resources/graphs/example_code_capacity_d3.json")
    val clockDivideBy = 2

    Config.sim
      .compile(MicroBlossomAxi4(config, clockDivideBy = clockDivideBy))
      // .compile(MicroBlossomAxiLite4(config, clockDivideBy = clockDivideBy))
      // .compile(MicroBlossomAxiLite4Bus32(config, clockDivideBy = clockDivideBy))
      .doSim("logic_validity") { dut =>
        dut.clockDomain.forkStimulus(period = 10)
        dut.slow.clockDomain.forkStimulus(period = 10 * clockDivideBy)

        val driver = dut.getSimDriver()

        val version = driver.read_32(8)
        printf("version: %x\n", version)
        assert(version == DualConfig.version)
        val contextDepth = driver.read_32(12)
        assert(contextDepth == config.contextDepth)
        val conflictChannels = driver.read_8(16)
        assert(conflictChannels == config.conflictChannels)
      }

  }

}

class MicroBlossomBusGeneratorConf(arguments: Seq[String]) extends ScallopConf(arguments) {
  val graph = opt[String](required = true, descr = "see ./resources/graphs/README.md for more details")
  val outputDir = opt[String](default = Some("gen"), descr = "by default generate the output at ./gen")
  val busType = opt[String](default = Some("Axi4"), descr = s"options: ${MicroBlossomBusType.options.mkString(", ")}")
  val languageHdl = opt[String](default = Some("verilog"), descr = "options: Verilog, VHDL, SystemVerilog")
  val baseAddress = opt[BigInt](default = Some(0), descr = "base address of the memory-mapped module, default to 0")
  // DualConfig
  val broadcastDelay = opt[Int](default = Some(0))
  val convergecastDelay = opt[Int](default = Some(1))
  val contextDepth = opt[Int](default = Some(1), descr = "how many contexts supported")
  val conflictChannels = opt[Int](default = Some(1), descr = "how many conflicts are reported at once")
  val dynamicWeights = opt[Boolean](default = Some(false), descr = "by default hard code the edge weights")
  val noAddDefectVertex = opt[Boolean](default = Some(false), descr = "by default support AddDefectVertex instruction")
  val supportOffloading = opt[Boolean](default = Some(false), descr = "support offloading optimization")
  val supportLayerFusion = opt[Boolean](default = Some(false), descr = "support layer fusion")
  val injectRegisters =
    opt[List[String]](
      default = Some(List()),
      descr = s"insert register at select stages: ${Stages().stageNames.mkString(", ")}"
    )
  val clockDivideBy = opt[Double](default = Some(2))
  verify()
  def dualConfig = DualConfig(
    filename = graph(),
    broadcastDelay = broadcastDelay(),
    convergecastDelay = convergecastDelay(),
    contextDepth = contextDepth(),
    conflictChannels = conflictChannels(),
    hardCodeWeights = !dynamicWeights(),
    supportAddDefectVertex = !noAddDefectVertex(),
    supportOffloading = supportOffloading(),
    supportLayerFusion = supportLayerFusion(),
    injectRegisters = injectRegisters()
  )
}

// sbt "runMain microblossom.MicroBlossomBusGenerator --help"
// (e.g.) sbt "runMain microblossom.MicroBlossomBusGenerator --graph ./resources/graphs/example_code_capacity_d3.json"
object MicroBlossomBusGenerator extends App {
  print("sbt \"runMain microblossom.MicroBlossomBusGenerator")
  args.foreach(printf(" %s", _))
  println("\"")
  val conf = new MicroBlossomBusGeneratorConf(args)
  val dualConfig = conf.dualConfig
  val genConfig = Config.argFolderPath(conf.outputDir())
  // note: deliberately not creating `component` here, otherwise it encounters null pointer error of GlobalData.get()....
  val mode: SpinalMode = conf.languageHdl() match {
    case "verilog" | "Verilog"             => Verilog
    case "VHDL" | "vhdl" | "Vhdl"          => VHDL
    case "SystemVerilog" | "systemverilog" => SystemVerilog
    case _ => throw new Exception(s"HDL language ${conf.languageHdl()} is not recognized")
  }
  genConfig
    .copy(mode = mode)
    .generateVerilog(
      MicroBlossomBusType.generateByName(conf.busType(), dualConfig, conf.clockDivideBy(), conf.baseAddress())
    )
}
