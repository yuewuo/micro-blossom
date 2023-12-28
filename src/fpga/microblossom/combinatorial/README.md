# Combinatorial Logic

All the logic here are organized as follows:

## `object` NAME

This is the implementation of the pure combinatorial logic.
By calling the `build` function, it builds the combinatorial logic without introducing new components.

## `case class` NAME

A class extending `Component`, wrapping the combinatorial logic in a separate module so that it could be shared.

## NAME`Test`

Test the correctness of the combinatorial logic when needed.
It also comes with an example which will be generated under the default `gen` folder.

## NAME`DelayEstimation`

Estimate the delay of the signal, using Vivado simulator.
This helps to understand the performance and also guide the design of the pipeline.
