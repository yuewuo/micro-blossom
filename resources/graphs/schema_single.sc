// DO NOT MODIFY MANUALLY!!!
// generate by https://app.quicktype.io/

package microblossom

import scala.util.Try
import io.circe.syntax._
import io.circe._
import cats.syntax.functor._

// For serialising string unions
given [A <: Singleton](using A <:< String): Decoder[A] =
  Decoder.decodeString.emapTry(x => Try(x.asInstanceOf[A]))
given [A <: Singleton](using ev: A <:< String): Encoder[A] =
  Encoder.encodeString.contramap(ev)

// If a union has a null in, then we'll need this too...
type NullValue = None.type

case class Coordinate(
    val positions: Seq[Position],
    val vertex_num: Long,
    val weighted_edges: Seq[WeightedEdges]
) derives Encoder.AsObject,
      Decoder

case class Position(
    val i: Double,
    val j: Double,
    val t: Double
) derives Encoder.AsObject,
      Decoder

case class WeightedEdges(
    val l: Long,
    val r: Long,
    val w: Long
) derives Encoder.AsObject,
      Decoder
