# Benchmark Decoding Speed

We evaluate the decoding speed by measuring both the decoding latency and the decoding CPU wall time.

The decoding latency is defined from the time the last round of syndrome is ready, to the time that the hardware
asserts a signal that indicate the decoding is finished, together with one or more bits that indicate the logical 
correction results (see src/cpu/blossom/resources.rs: `MicroBlossomSingle::parity_reporters`).
If offloading is enabled, both the CPU and the hardware accelerator contribute to this reported parity of logical
correction.
If no CPU interaction is needed, i.e., all the syndrome is handled by the hardware accelerator solely,
then the latency could be very small, up to a few clock cycles.
However, once CPU is needed to decode the results that are beyond the capability of hardware accelerators, then
the latency will be a few hundreds nanoseconds per interaction due to the clock domain crossing delay inside the CPU
as well as the time to run the blossom algorithm on the CPU.

The CPU wall time is defined from the time we start decoding to the time decoding is finished from the view of the CPU.
This metrics is recorded for the use of analysis, but it does not directly reflect the decoding latency.
This is because the CPU is capable of knowing when the syndrome is ready and issue a read command well before the syndrome
is ready.
The hardware will stall if the syndrome requested is not ready yet, and thus stall the memory bus.
Once the syndrome is ready, it is loaded to the hardware accelerator and generate an obstacle.
If there is no obstacle, the decoding is finished and there is no need to wait for CPU responses.
This is why the decoding latency could be very low if CPU do not need to handle anything.
However, from the perspective of CPU, the decoding wall time could be longer.


## Evaluation design

Since we want the hardware runtime to be manageable, i.e., no larger than tens of minutes, it's better to run
a small batch of data and obtain the results.

Let's assume each decoding instance takes 10us, including everything.
We have B=10^7 samples: it will run for about 100s, about 1 min.
Each sample contains about 2 bytes * number of defects, which is about 66 bytes for d=15 circuit-level noise.
This corresponds to 660MB file.
Thus, it is surprising that generating the file could be way more expensive than actually running it on hardware.


## Evaluation Plans

### 1. Decoding time changing with code distance and physical error rate

We plan to have a figure whose X axis is physical error rate, and Y axis is the average decoding latency.
We will draw multiple curves, each corresponds to one code distance.

There will be three figures: 

2024.6.8
change circuit_level_batch/fusion/no_offloading to use 10^6 samples Each, to provide sufficient accuracy.
run the following

```sh
python3 circuit_level_fusion/run.py ; python3 circuit_level_batch/run.py ;  python3 circuit_level_no_offloading/run.py ; python3 measurement_round/run.py ; python3 measurement_rate/run.py ; python3 distribution/run.py
```
