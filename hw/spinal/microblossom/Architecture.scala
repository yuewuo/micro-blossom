package microblossom

import spinal.core._

/*
 * # Instruction Set Architecture (ISA) of dual accelerator
 *
 * The instruction set of dual accelerator: each instruction is 32-bits wide:
 * -------------------------------------------------------------------------------------------------
 * |31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10  9  8  7  6  5  4  3  2  1  0|
 * |                  Node[14:0]                |Speed|                   0                  |2'b00| SetSpeed
 * |                  Node[14:0]                |                  Blossom[14:0]             |2'b01| SetBlossom
 * |               Vertex_1[14:0]               |                 Vertex_2[14:0]             |2'b11| Match
 * |                                        Length[29:0]                                     |2'b10| Grow
 * |                                                          RegionPreference   | 3'b000 | 3'b100 | FindObstacle
 * |                                                              Address        | 3'b001 | 3'b100 | ClearAccumulator
 * |                                                             EdgeIndex       | 3'b010 | 3'b100 | AccumulateEdge
 * |                                     Reserved                                | 3'b011 | 3'b100 | Reserved
 * |                                         0                                   | 3'b100 | 3'b100 | Reset
 * |                  Time[14:0]                |           Channel[11:0]        | 3'b101 | 3'b100 | LoadSyndrome
 * -------------------------------------------------------------------------------------------------
 *
 *
 * The return command is also 32-bits wide, but some messages are splitted into two
 * -------------------------------------------------------------------------------------------------
 * |31 30 29 28 27 26 25 24 23 22 21 20 19 18 17 16 15 14 13 12 11 10  9  8  7  6  5  4  3  2  1  0|
 * |                                        Length[29:0]                                     |2'b00| NonZeroGrow
 * |                Node_1[14:0]                |                  Node_2[14:0]              |2'b01| Conflict(part 1)
 * |               Touch_1[14:0]                |                 Touch_2[14:0]              |2'b01| Conflict(part 2)
 * |               Vertex_1[14:0]               |                 Vertex_2[14:0]             |2'b01| Conflict(part 3)
 * |                 Node[14:0]                 |                  Touch[14:0]               |2'b10| ConflictVirtual(part 1)
 * |                Vertex[14:0]                |               VirtualVertex[14:0]          |2'b10| ConflictVirtual(part 2)
 * -------------------------------------------------------------------------------------------------
 *
 *
 * The above interface defines the interface defined by the dual accelerator, but not the internal message type
 * For example, an implementation of a dual accelerator can use as few bits as possible for internal messaging
 * Also, it is possible to pause the input if it takes more than 1 clock cycle, e.g., FindObstacle and Match
 *
 * # Optimization 1: primal offloading
 *
 * If we consider a physical error rate of 0.001, then roughly (1-0.003)^24 = 93% of the matchings are simple matchings.
 * By simple matching, it means the incident vertices are the two (non-matched) defect vertices of the conflicting edge.
 * Thus, we can simply match them in a single clock cycle; the two defect are then marked as simple_match so they will not
 * involve in other simple matchings in the future. Whenever a simple_match vertex has a conflict with others, it reports
 * the simple matching instead of a single defect. Normally this simple matching will be attached to an alternating tree.
 *
 *
 */

object OpCode extends SpinalEnum {
  val Grow, SetSpeed, SetBlossom = newElement()
  val Match = newElement()
}

case class Instruction(MessageBits: Int) extends Bundle {
  val opcode = OpCode()
  val payload = Bits(MessageBits bits)
}
