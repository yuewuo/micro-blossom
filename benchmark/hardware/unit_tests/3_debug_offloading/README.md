The offloading module did not work as expected.
It seems to offload only a very small portion of the syndrome, which does not agree with the simulation.

There are two configuration differences between the hardware test and the simulation though.
First, the number of noisey measurement rounds.
The simulation was T=d but the hardware is T=d-1.
Second, the max_half_weight value; the simulation was using the default value of 5000 but the hardware is using 7.
I changed the simulation configuration and rerun the benchmark in `benchmark/behavior/offloading_rate_virtual`.
Waiting for the results...

At the same time, I use two small instances to debug the case, hopefully figure out the reason.
They both use d=3 code without stage injection and runs at 50MHz.
