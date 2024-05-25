// see /resources/graphs/README.md for more information

package microblossom.util

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._

@ConfiguredJsonCodec
case class SingleGraph(
    var positions: Seq[Position],
    var vertex_num: Long,
    var weighted_edges: Seq[WeightedEdge],
    var virtual_vertices: Seq[Long],
    var vertex_binary_tree: BinaryTree,
    var edge_binary_tree: BinaryTree,
    var vertex_edge_binary_tree: BinaryTree,
    var vertex_max_growth: Seq[Long],
    var offloading: Seq[Offloading]
)

@ConfiguredJsonCodec
case class Position(
    var i: Double,
    var j: Double,
    var t: Double
)

@ConfiguredJsonCodec
case class WeightedEdge(
    var l: Long,
    var r: Long,
    var w: Long
)

@ConfiguredJsonCodec
case class BinaryTree(
    var nodes: Seq[BinaryTreeNode]
)

@ConfiguredJsonCodec
case class BinaryTreeNode(
    var p: Option[Long], // parent
    var l: Option[Long], // left
    var r: Option[Long] // right
)

@ConfiguredJsonCodec
case class Offloading(
    var dm: Option[DefectMatch],
    var vm: Option[VirtualMatch],
    var fm: Option[FusionMatch]
)

@ConfiguredJsonCodec
case class DefectMatch(
    var e: Long // edge_index
)

@ConfiguredJsonCodec
case class VirtualMatch(
    var e: Long, // edge_index
    var v: Long // virtual_vertex
)

@ConfiguredJsonCodec
case class FusionMatch(
    var e: Long, // edge_index
    var c: Long // conditioned_vertex
)

object SingleGraph {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object Position {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object WeightedEdge {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object BinaryTree {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object BinaryTreeNode {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object Offloading {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object DefectMatch {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object VirtualMatch {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object FusionMatch {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
