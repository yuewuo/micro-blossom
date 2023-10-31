package microblossom

/*
 * A host of scala program that talks with a parent process through TCP
 *
 * The parent process needs to start a TCP server and listen to a specific address and port;
 * the address and port information is passed to the Scala program via command line arguments.
 * When started, the program will try to connect to the port and then fetch a JSON that
 * describes the decoding graph; it then constructs a dual accelerator and start simulator.
 *
 */

import java.io._
import java.net._

// sbt "runMain microblossom.DualHost localhost 4123"
object DualHost extends App {
  if (args.length != 2) {
    println("usage: <address> <port>")
    sys.exit(1)
  }
  val hostname = args(0)
  val port = Integer.parseInt(args(1))
  println(hostname)
  println(port)
  val socket = new Socket(hostname, port)
  try {
    val outStream = new PrintWriter(socket.getOutputStream, true)
    val inStream = new BufferedReader(new InputStreamReader(socket.getInputStream))

    outStream.println("DualHost v0.0.1, ask for decoding graph")

    val response = inStream.readLine()
    println("Server response: " + response)
  } catch {
    case e: Exception => e.printStackTrace()
  } finally {
    socket.close()
  }

}
