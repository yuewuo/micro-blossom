# Explore Maximum clock frequency of the slow clock domain in d=9 circuit-level noise

We'll use 250MHz Axi4 bus frequency as verified in ../axi4_with_small_code.
The slow clock domain will search starting from 200MHz check what's the maximum frequency.

Here we enable all the optimization including offloading and layer fusion.
We choose d=9 because this is the major result that we want to demonstrate.
Other code distance will use the same configuration but slightly suboptimal.
