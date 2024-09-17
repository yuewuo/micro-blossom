# Micro Blossom

A highly configurable hardware-accelerated Minimum-Weight Perfect Matching (MWPM) decoder for Quantum Error Correction (QEC).

Micro Blossom is a heterogenous architecture that solves **exact** MWPM decoding problem in sub-microsecond latency by
taking advantage of vertex and edge-level fine-grained hardware acceleration.
At the heart of Micro Blossom is an algorithm (equivalent to the original blossom algorithm) specifically optimized for resource-efficient RTL (register transfer level) implementation, with compact combinatorial logic and pipeline design.
Given arbitrary decoding graph, Micro Blossom automatically generates a hardware implementation (either Verilog or VHDL depending on your needs) that accelerates the solution finding.
The heterogenous architecture of Micro Blossom is shown below:

![](tutorial/src/img/architecture.png)

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
