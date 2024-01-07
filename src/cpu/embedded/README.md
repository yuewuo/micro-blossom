# Embedded firmware

This project builds firmwares to be run on embedded CPUs like RiscV, Cortex R5 or A72, etc.
The binary could be an executable binary or a static library, etc, depending on the CPU and purpose.
For RiscV, we build the final executable firmware that can be loaded directly into the memory and execute;
for the CPUs in the PS of Xilinx FPGAs, we build static library and then let the Xilinx Vitis toolchain to generate the image.
The different building process is provided in the Makefile and useful information about them are printed out.

## RiscV

First we need the RiscV toolchain

```sh
git clone https://github.com/riscv/riscv-gnu-toolchain
cd riscv-gnu-toolchain
git submodule update --init --recursive
# also install any dependencies, see https://github.com/riscv/riscv-gnu-toolchain for more details
./configure --prefix=$HOME/.riscv --enable-multilib


```
