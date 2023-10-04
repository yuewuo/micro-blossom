ThisBuild / version := "1.0"
ThisBuild / scalaVersion := "2.13.12"
ThisBuild / organization := "org.yecl"

val spinalVersion = "1.9.3"
val spinalCore = "com.github.spinalhdl" %% "spinalhdl-core" % spinalVersion
val spinalLib = "com.github.spinalhdl" %% "spinalhdl-lib" % spinalVersion
val spinalIdslPlugin = compilerPlugin("com.github.spinalhdl" %% "spinalhdl-idsl-plugin" % spinalVersion)

lazy val microblossom = (project in file("."))
  .settings(
    Compile / scalaSource := baseDirectory.value / "src" / "fpga",
    Test / scalaSource := baseDirectory.value / "src" / "fpga",
    scalacOptions ++= Seq("-Ymacro-annotations"),
    libraryDependencies ++= Seq(
      spinalCore,
      spinalLib,
      spinalIdslPlugin,
      "org.scalatest" %% "scalatest" % "3.2.5",
      "io.circe" %% "circe-core" % "0.14.3",
      "io.circe" %% "circe-generic" % "0.14.3",
      "io.circe" %% "circe-parser" % "0.14.3",
      "io.circe" %% "circe-generic-extras" % "0.14.3"
    )
  )

fork := true
