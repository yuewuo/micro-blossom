// generated by https://app.quicktype.io/ but modified to use Scala2 json codec
// see /resources/graphs/README.md for more information

package microblossom.util

import io.circe._
import io.circe.generic.extras._
import io.circe.generic.semiauto._

@ConfiguredJsonCodec
case class SingleGraph(
    val positions: Seq[Position],
    val vertex_num: Long,
    val weighted_edges: Seq[WeightedEdges]
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

object SingleGraph {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object Position {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
object WeightedEdges {
  implicit val config: Configuration = Configuration.default.withSnakeCaseMemberNames
}
