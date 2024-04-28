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

OK now I'm sure that there is bug in the implementation of the offloading module.
None of the single edges benefit from the offloading, so I need to check the Scala implementation with
comparison to the Rust implementation.
The latter indeed shows benefit.

Let me first check if the parameter is successfully passed in.

```sh
# under git root folder
sbt assembly
java -cp target/scala-2.12/microblossom.jar microblossom.MicroBlossomGenerator --support-offloading --output-dir benchmark/hardware/unit_tests/3_debug_offloading/hardware/offloaded_true/offloaded_true_verilog --graph benchmark/hardware/unit_tests/3_debug_offloading/hardware/prepare.json
java -cp target/scala-2.12/microblossom.jar microblossom.MicroBlossomGenerator --output-dir benchmark/hardware/unit_tests/3_debug_offloading/hardware/offloaded_false/offloaded_false_verilog --graph benchmark/hardware/unit_tests/3_debug_offloading/hardware/prepare.json
```

Yes it is.

Then I'll run the simulator to see if the same problem persists.
Here I'm assuming all the data are prepared, e.g. already run `run.py` so that the `embedded.defects` file is ready.

```sh
cd src/cpu/blossom
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding SUPPORT_OFFLOADING=1 cargo run --release --bin embedded_simulator -- ../../../benchmark/hardware/unit_tests/3_debug_offloading/hardware/prepare.json
```

Well, as expected and fortunately, this RTL-level simulator agrees with the hardware.
Now we can debug in pure software.
The conclusion is basically the Scala implementation did not fully replicate that of the Rust implementation.
Or some bug fixes in Rust did not reflect in Scala.
Anyway we need to figure that out.

To simplify debugging, we only test edge 0 and 1, which corresponds to
```json
{"defect_vertices":[0],"erasures":[],"dynamic_weights":[]}
{"defect_vertices":[0,3],"erasures":[],"dynamic_weights":[]}
```

Modify this in `run/run.syndromes` and delete `run.defects` and regenerate it.
Also copy this to `/src/cpu/embedded/embedded.defects` so that the software uses these set of syndromes.

Now let's run

```sh
cd src/cpu/blossom
WITH_WAVEFORM=1 EMBEDDED_BLOSSOM_MAIN=benchmark_decoding SUPPORT_OFFLOADING=1 cargo run --release --bin embedded_simulator -- ../../../benchmark/hardware/unit_tests/3_debug_offloading/hardware/prepare.json
# check the waveform
# we know that edges 0 and 1 should have been offloaded
gtkwave ../../../simWorkspace/MicroBlossomHost/benchmark_decoding/hosted.fst
```

After examining the internal signals, it seems like the offloading is asserted and the obstacle is indeed suppressed.
However, the reported obstacle has some delay in the tree and it seems like AXI4 host mistakenly capture the old obstacle.
This can be fixed by changing `currentEntry := historyEntries(dualConfig.readLatency)` with `readoutLatency (=dualConfig.readLatency+1)`.
However, while this fixes the offloading behavior, it breaks an existing sanity check test:

```sh
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom cargo run --release --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_d3.json
```
I think replacing `dualConfig.readLatency` with `readoutLatency` does make sense, and the breaking change may come from other bugs in the logic.
I'll have to dig into the logic and figure out solutions.
Also, it will be helpful to come up with a sequence of unit tests, including those that enables the offloading.
It's always beneficial to test the behavior for known cases, so that we don't accidentally make breaking changes.
