package microblossom.util

import microblossom._
import sys.process._
import scala.io.Source
import scala.collection.mutable
import scala.util.Try
import scala.util.Random
import scala.util.matching.Regex
import java.nio.file.{Paths, Files}
import java.nio.charset.StandardCharsets

case class ResourceTable(
    var index: String = null,
    var name: String = null,
    var tableLines: mutable.ArrayBuffer[String] = null
) {
  def print() = {
    println(s"$index $name")
    println(tableLines.mkString("\n"))
  }
}

case class ResourceReport(filepath: String) {
  val tables = mutable.Map[String, ResourceTable]()

  def primitivesTable = tables("Primitives")

  val source = Source.fromFile(filepath)
  try {
    var isLastTitle = false
    var lastTitle = ""
    var lastIndex = ""
    var lineIterator = source.getLines().buffered
    while (lineIterator.hasNext) {
      val line = lineIterator.next().trim
      val titlePattern: Regex = """(\d+.\d*) ([\S\s]*)""".r

      if (line.startsWith("---")) {
        if (isLastTitle) {
          while (lineIterator.hasNext && !lineIterator.head.startsWith("+--")) {
            lineIterator.next()
          }
          var tableLines = mutable.ArrayBuffer[String]()
          tableLines.append(lineIterator.next())
          while (lineIterator.hasNext && !lineIterator.head.startsWith("+--")) {
            tableLines.append(lineIterator.next())
          }
          tableLines.append(lineIterator.next())
          while (lineIterator.hasNext && !lineIterator.head.startsWith("+--")) {
            tableLines.append(lineIterator.next())
          }
          tableLines.append(lineIterator.next())
          tables(lastTitle) = ResourceTable(lastIndex, lastTitle, tableLines)
        }
      }

      isLastTitle = false
      line match {
        case titlePattern(index, title) =>
          isLastTitle = true
          lastTitle = title
          lastIndex = index
        case _ => // no nothing
      }
    }
  } finally {
    source.close()
  }
}
