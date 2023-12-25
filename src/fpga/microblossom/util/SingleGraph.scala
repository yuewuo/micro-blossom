// see /resources/graphs/README.md for more information

package microblossom.util

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._

@ConfiguredJsonCodec
case class SingleGraph(
    val positions: Seq[Position],
    val vertex_num: Long,
    val weighted_edges: Seq[WeightedEdges],
    val virtual_vertices: Seq[Long],
    val vertex_binary_tree: BinaryTree,
    val edge_binary_tree: BinaryTree,
    val vertex_edge_binary_tree: BinaryTree,
    val vertex_max_growth: Seq[Long],
    val offloading: Seq[Offloading]
)

@ConfiguredJsonCodec
case class Position(
    val i: Double,
    val j: Double,
    val t: Double
)

@ConfiguredJsonCodec
case class WeightedEdges(
    val l: Long,
    val r: Long,
    val w: Long
)

@ConfiguredJsonCodec
case class BinaryTree(
    val nodes: Seq[BinaryTreeNode]
)

@ConfiguredJsonCodec
case class BinaryTreeNode(
    val parent: Option[Long],
    val left: Option[Long],
    val right: Option[Long]
)

@ConfiguredJsonCodec
case class Offloading(
    val DM: Option[DefectMatch],
    val VM: Option[VirtualMatch]
)

@ConfiguredJsonCodec
case class DefectMatch(
    val edge_index: Long
)

@ConfiguredJsonCodec
case class VirtualMatch(
    val edge_index: Long,
    val virtual_vertex: Long
)

object SingleGraph {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object Position {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object WeightedEdges {
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
