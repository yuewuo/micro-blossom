# Explore Maximum clock frequency of the slow clock domain in d=9 circuit-level noise

We'll use 250MHz Axi4 bus frequency as verified in ../axi4_with_small_code.
The slow clock domain will search starting from 200MHz check what's the maximum frequency.

Here we enable all the optimization including offloading and layer fusion.
We choose d=9 because this is the major result that we want to demonstrate.
Other code distance will use the same configuration but slightly suboptimal.

Experience: 
The best option is to inject `execute` and `update` registers, with 0 boardcast delay and 1 convergecast delay.
By injecting the two registers, it improves from 46MHz (0 register) to 62MHz (1 regiser) to 91MHz (2 registers).
Injecting more registers does not help the frequency by much, probably because it's bounded by the connectivity.
This is reasonable because the 3D graph cannot be perfectly fitted into a 2D device, thus non-local behavior exists.
