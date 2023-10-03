package microblossom

import spinal.core._
import microblossom._

// // Hardware definition
// case class Vertex(VertexBits: Int, VertexIndex: Int) extends Component {
//   val io = new Bundle {
//     val instruction = in(VertexInstruction(VertexBits))
//   }

//   val counter = Reg(UInt(8 bits)) init 0

//   // decode stage
//   val command = RegInit(VertexInstructionType.None)
//   command := io.instruction.command
//   switch(command) {
//     is(VertexInstructionType.Grow) {}
//   }

//   // io.state := counter
//   // io.flag := (counter === 0) | io.cond1
// }

// object VertexVerilog extends App {
//   // an example of vertex
//   Config.spinal.generateVerilog(Vertex(8, 1))
// }

// object VertexVhdl extends App {
//   Config.spinal.generateVhdl(Vertex(8, 1))
// }
