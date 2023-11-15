# Distributed MWPM Decoder System

Distributed MWPM decoder for Quantum Error Correction

## Project Structure

- src: source code
  - fpga: the FPGA source code, including generator scripts
  - cpu: the CPU source code
- projects: Vivado projects


## Installation

For FPGA development and testing, we use [Verilator](https://verilator.org/guide/latest/install.html).
We pin to a specific version 5.014 to avoid incompatibility.

```sh
sudo apt install gtkwave
sudo apt install git help2man perl python3 make autoconf g++ flex bison ccache
sudo apt install libgoogle-perftools-dev numactl perl-doc
sudo apt-get install libfl2  # Ubuntu only (ignore if gives error)
sudo apt-get install libfl-dev  # Ubuntu only (ignore if gives error)
sudo apt-get install zlibc zlib1g zlib1g-dev  # Ubuntu only (ignore if gives error)

git clone https://github.com/verilator/verilator   # Only first time
cd verilator
git pull
git checkout v5.014      # pin to this specific version 

autoconf         # Create ./configure script
./configure      # Configure and create Makefile
make -j `nproc`  # Build Verilator itself (if error, try just 'make')
sudo make install
```

### Install VexRiscV

```sh
git clone git@github.com:SpinalHDL/VexRiscv.git --recursive
```

### Install OpenOCD for VexRiscV

I include the installation for both Ubuntu and MacOS. MacOS doesn't seem to work...: `Error: target 'fpga_spinal.cpu0' init failed`
when I run `openocd -c 'set VEXRISCV_YAML ../VexRiscv/cpu0.yaml' -f tcl/target/vexriscv_sim.cfg`.

**Update**: OpenOCD uses GDB instead of LLDB, but GDB does not support apple silicon, only x86. Thus, it is impossible to 
interactively debug the CPU with M1 Macs. However, the verilator and VexRiscV/SpinalHDL runs on M1 Macs without a problem.
We can debug the design on a x86 Linux machine and do other staffs on Macs.

```sh
git clone git@github.com:SpinalHDL/openocd_riscv.git
cd openocd_riscv

# for Ubuntu
sudo apt-get install libtool automake texinfo libusb-1.0-0-dev libusb-dev libyaml-dev pkg-config
# for MacOS (@Yue 2023.10.11)
brew install libtool automake libusb libyaml pkg-config texinfo

./bootstrap

# for Ubuntu
./configure --enable-ftdi --enable-dummy --enable-openjtag
# for MacOS (@Yue 2023.10.11, must enable openjtag for simulation)
LDFLAGS="-L/opt/homebrew/lib" CPPFLAGS="-I/opt/homebrew/include" ./configure --enable-ftdi --enable-dummy --disable-werror --enable-openjtag

make -j10
sudo make install
```

### Install RiscV toolchain

```sh
git clone https://github.com/riscv/riscv-gnu-toolchain

# for Ubuntu
sudo apt-get install autoconf automake autotools-dev curl python3 python3-pip libmpc-dev libmpfr-dev libgmp-dev gawk build-essential bison flex texinfo gperf libtool patchutils bc zlib1g-dev libexpat-dev ninja-build git cmake libglib2.0-dev
# for MacOS
brew install python3 gawk gnu-sed gmp mpfr libmpc isl zlib expat texinfo flock

mkdir $HOME/riscv
./configure --prefix=$HOME/riscv --with-arch=rv32ia --with-abi=ilp32
./configure --prefix=$HOME/riscv --with-arch=rv32ia --with-abi=ilp32 --disable-gdb --enable-llvm # for MacOS

make -j10
```

### **Only on Linux** Simulation

```sh
# In the VexRiscv repository (`make run` should show `Boot` and then hang there until the OpenOCD is connected)
sbt "runMain vexriscv.demo.GenFull"
cd src/test/cpp/regression
make run DEBUG_PLUGIN_EXTERNAL=yes

# In the openocd repository, after building it =>
src/openocd -c "set VEXRISCV_YAML ../VexRiscv/cpu0.yaml" -f tcl/target/vexriscv_sim.cfg

# Run GDB session
riscv32-unknown-elf-gdb VexRiscvRepo/src/test/resources/elf/uart.elf
target remote localhost:3333
monitor reset halt
load
continue
# then a sequence of messages should be print to the first terminal
```

### Install HLS toolchain

```sh
# install llvm 15.x
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 15
```

### Install YoSys synthesizer

It's easiest to install a pre-built binary from [OSS CAD](https://github.com/YosysHQ/oss-cad-suite-build).

## Build

### Hardware

### Binary for RiscV CPU

```sh
cd src/cpu/embedded
make
```

## Developer Notes

### How to see the assembly code of a given function

```sh
cargo install cargo-show-asm
cargo asm --rust --target riscv32i-unknown-none-elf --bin embedded_blossom
cargo asm --rust --target riscv32i-unknown-none-elf --lib 
```

### How to run scala test

```sh
sbt test  # test all
sbt 'testOnly *DualConfigTest'
```

### Download SpinalHDL library for development

```sh
git clone git@github.com:SpinalHDL/SpinalHDL.git
cd SpinalHDL
git checkout v1.9.3
```

### How to estimate LUT usage of a design

```sh
cargo run --bin micro-blossom  # generate the graph json files in resources/
sbt "runMain microblossom.DualAcceleratorExamples"  # generate verilog files in gen/example_*/
yosys -s src/fpga/yosys/synthesize.ys  # generate the report in gen/DualAccelerator.json
```

For example, for a phenomenological noise model with d=11, we get the usage information of

```
   Number of wires:             1912284
   Number of wire bits:         9305386
   Number of public wires:      410423
   Number of public wire bits:  5560940
   Number of memories:               0
   Number of memory bits:            0
   Number of processes:              0
   Number of cells:             2991070
     BUFG                            1
     CARRY4                      84934
     FDCE                        18300
     FDRE                       532064
     FDSE                        59544
     IBUF                           35
     INV                         17796
     LUT1                        32394
     LUT2                       273502
     LUT3                       283016
     LUT4                       135305
     LUT5                       388120
     LUT6                       475984
     MUXF7                      534031
     MUXF8                      140376
     MUXF9                       15573
     OBUF                           95
```

When implementing on Vivado, the resource usage is as follows:

```
phenomenological d=3: 18298 LUT (7.94%)
```

## Known Issues

The reset signal generated by SpinalHDL is asynchronous reset.
Need to specify the clock domain in the top level design to make it synchronous.

I wanted to use the latest Scala but it doesn't seem to be compatible with VexRiscV code base.
Official VexRiscV uses Scala 2.11.12, but in this project I want to use `circe` which requires at least 2.12 or 2.13.
I tried to bump up the VexRiscV version to 2.13.12 but it shows a lot of errors.
Bumping to 2.12.12 is fine, so we can stay here for a while.
The updated repo is at [git@github.com:yuewuo/VexRiscv.git](git@github.com:yuewuo/VexRiscv.git).
The tested commands are: 

```sh
sbt "runMain vexriscv.demo.Briey"  # try to build briey
VEXRISCV_REGRESSION_SEED=42 VEXRISCV_REGRESSION_LINUX_REGRESSION=no VEXRISCV_REGRESSION_ZEPHYR_COUNT=0 sbt "testOnly vexriscv.TestIndividualFeatures"
```

## References

[Blog: Rust on Risc-V, by Craig J Bishop](https://craigjb.com/2020/01/22/ecp5/)
