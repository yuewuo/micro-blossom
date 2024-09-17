# Micro Blossom

A highly configurable hardware-accelerated Minimum-Weight Perfect Matching (MWPM) decoder for Quantum Error Correction (QEC).

Paper coming soon!!! Stay tuned!!!

Micro Blossom is a heterogenous architecture that solves **exact** MWPM decoding problem in sub-microsecond latency by
taking advantage of vertex and edge-level fine-grained hardware acceleration.
At the heart of Micro Blossom is an algorithm (equivalent to the original blossom algorithm) specifically optimized for resource-efficient RTL (register transfer level) implementation, with compact combinatorial logic and pipeline design.
Given arbitrary decoding graph, Micro Blossom automatically generates a hardware implementation (either Verilog or VHDL depending on your needs) that accelerates the solution finding.
The heterogenous architecture of Micro Blossom is shown below:

![](./tutorial/src/img/architecture.png)

## Benchmark Highlights

**14x latency reduction**: On a surface code of code distance $d=9$ and physical error rate of $p=0.001$ circuit-level noise model, we reduce the average
latency from $5.1 \mu s$ using Parity Blossom on CPU (Apple M1 Max) to $367 ns$ using Micro Blossom on FPGA (VMK180), a 14x reduction in **latency**.
Although [Sparse Blossom (PyMatching V2)](https://github.com/oscarhiggott/PyMatching) is generally faster in this case, it still incurs $3.2 \mu s$ latency considering the $2.4 \mu s$ pure calculation and at least $0.8 \mu s$ CPU-hardware communication latency (PCIe) when using powerful CPUs.

**better effective error rate**: on various code distances than both Helios (hardware UF decoder, which runs even faster but loses accuracy) and Parity Blossom (running on M1 Max). It is only at very large code distances ($d \ge 13$) and physical error rate ($p \ge 0.5\%$) that Helios starts to outperform Micro Blossom. Such complexity is inherent to the optimality of blossom algorithm though. If one want to opt for decoding speed rather than accuracy, one can tune between MWPM and UF using Micro Blossom (currently not supported but could be easily ported from this [`max_tree_size` feature](https://github.com/yuewuo/fusion-blossom/issues/31) in the Fusion Blossom library)

**latency distribution with exponential tail**: we observe an exponential tail of the latency distribution. The software implementation has similar behavior but it is affected by cache misses at those rare but complicated cases. In Micro Blossom, although we still have a CPU, the memory footprint is much smaller due to the fact that the decoding graph is not stored in the CPU at all. The CPU only has an active memory region that scales with $O(p^2 |V|)$ (Yes!!! not $O(p|V|)$ which is the average number of defect vertices, but rather $O(p^2 |V|)$, further reducing the memory size).

Note that an improvement of latency is generally harder than improvement of throughput, because the latter can be achieved by
increasing the number of cores using coarse-grained parallelism but latency is bounded by how much the algorithm is sequential at its core. Given the complexity and sequential nature of the blossom
algorithm, it was even believed in the community that hardware acceleration of exact MWPM decoding is impractical. Yet we re-design the algorithm
to exploit the finest parallelism possible (vertex and edge parallelism), and achieves such a huge improvement in latency.
Note that this doesn't mean we are sacrificing the decoding throughput: in fact, thanks to the pipeline design, the hardware accelerator
has a huge throughput capability that supports decoding of 110 logical qubits ($d=9, p=0.001$) while achieving the throughput requirement of 1 million measurement
rounds per second.
While Micro Blossom does use more resources (152k LUT) than [Helios](https://github.com/NamiLiy/Helios_scalable_QEC) (94k LUT),
the resource usage per logical qubit is 1.4k LUT, lower than the 2.1k LUT per logical qubit for Helios.
This is due to the more efficient CPU-hardware collaboration of Micro Blossom where the hardware focuses on massive yet simple parallel computation while the
CPU focuses on complicated yet rare computation.

![](./tutorial/src/img/benchmark.png)

For people with concern about why we evaluate the average latency rather than worst-case latency: we believe only average case matters for several reasons below. Note that when we evaluate the decoding latency distribution, we accumulate 1000 logical errors (2.5e8 samples in total) to make sure we capture the latencies with probability at or even below the logical error rate $p_L$. This doesn't change the average latency value by much though. For all other cases that only require average latency value, we just run 1e5 samples.

- Adding "idle QEC cycle" support at the lowest control layer is not hard, and is favorable for various reasons beyond QEC decoding: for example, some logical feedforward branches has longer execution path and we could just let other uninvovled logical qubits run their idle cycles while waiting for a longer branch to run.
- Mathematically, only average case matters. Our ultimate goal is that the overall logical error rate (including the added idle time due to decoding latency and feedforward) is low. Let's calculate the overall logical error rate including the latency-induced idle errors: Suppose the latency distribution is $P(L)$, then $\int_0^\infty P(L) dL = 1$ and $\int_0^\infty P(L) L dL = \bar{L}$ (aka average latency). Let's calculate the overall logical error rate. $p_L = \int_0^\infty P(L) p_{L0} (1 + L/d) dL = p_{L0} (1 + \bar{L}/d)$. This result is also intuitive, because logical error rate is a also a statistical number that is linearly related to the latency.
- It is difficult to scale up QEC decoding to distributed systems while meeting hard deadline requirements, even though there are existing work that claims hard real-time for a simpler memory experiments at extremely low physical error rate or code distances. We believe in the long run, all decoders will face the problem of not being able to achieve hard $1 \mu s$ deadline but it doesn't matter that much according to the two points above.

## Project Structure

- src: source code
  - fpga: the FPGA source code, including generator scripts
    - microblossom: Scala code for MicroBlossom module using SpinalHDL library
    - Xilinx: Xilinx project build scripts
  - cpu: the CPU source code
    - blossom: code for development and testing on a host machine
    - blossom-nostd: nostd code that is designed to run in embedded environment but can also run in OS
    - embedded: build binary for embedded system
- benchmark: evaluation results that can be reproduced using the scripts included (and VMK180 Xilinx evaluation board)
  - behavior: CPU simulation, with exactly the same RTL logic
  - hardware: evaluation on hardware
    - bram_speed: understand the CPU-FPGA communication cost under various clock frequencies
    - decoding_speed: evaluate the decoding latency under various conditions and its distribution on real VMK180 hardware
    - resource_estimate: post-implementation resource usage
