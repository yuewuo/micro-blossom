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

## NAME`Estimation`

Estimate the delay of the signal, using Vivado simulator.
This helps to understand the performance and also guide the design of the pipeline.


# Observations

Usually the routing takes the most of the time instead of the LUTs.
LUT takes as small as 0.04ns to have the results ready, but it takes 0.28ns to 0.59ns to propagate between LUTs.
We should expect the propagation between the combinatorial logics are around 0.5ns each.
Even for the circuit-level noise, each combinatorial logic usually takes only 0.67ns at most.
That being said, the propagating delay is very significant and it should be reflected in the diagram.

