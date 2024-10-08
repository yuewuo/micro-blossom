all: armv7r aarch64 riscv

clean:
	cargo clean

riscv: riscv-release riscv-debug

# RISCV_TARGET ?= riscv32imac-unknown-none-elf
RISCV_TARGET ?= riscv32i-unknown-none-elf
FEATURES ?= 

riscv-release:
	cargo build --release --features="riscv,$(FEATURES)" --target $(RISCV_TARGET)
	riscv32-unknown-elf-objdump -d target/$(RISCV_TARGET)/release/embedded_blossom > target/$(RISCV_TARGET)/release/embedded_blossom.s
	riscv32-unknown-elf-strip target/$(RISCV_TARGET)/release/embedded_blossom
	riscv32-unknown-elf-objcopy -O binary target/$(RISCV_TARGET)/release/embedded_blossom target/$(RISCV_TARGET)/release/embedded_blossom.bin
	ls -al target/$(RISCV_TARGET)/release/embedded_blossom.bin

riscv-debug:
	cargo build --features="riscv,$(FEATURES)" --target $(RISCV_TARGET)
	riscv32-unknown-elf-objdump -d target/$(RISCV_TARGET)/debug/embedded_blossom > target/$(RISCV_TARGET)/debug/embedded_blossom.s
	riscv32-unknown-elf-strip target/$(RISCV_TARGET)/debug/embedded_blossom
	riscv32-unknown-elf-objcopy -O binary target/$(RISCV_TARGET)/debug/embedded_blossom target/$(RISCV_TARGET)/debug/embedded_blossom.bin
	ls -al target/$(RISCV_TARGET)/debug/embedded_blossom.bin

Xilinx: armv7r aarch64

# use by Cortex R5F as Xilinx Versal RPU
armv7r: armv7r-release armv7r-debug

ARMV7R_TARGET ?= armv7r-none-eabihf

armv7r-release:
	cargo build --lib --release --features="$(FEATURES)" --target $(ARMV7R_TARGET)

armv7r-debug:
	cargo build --lib --features="$(FEATURES)" --target $(ARMV7R_TARGET)

# use by Cortex A53 or A72 as Xilinx Versal APU
aarch64: aarch64-release aarch64-debug

AARCH64_TARGET ?= aarch64-unknown-none

aarch64-release:
	cargo build --lib --release --features="$(FEATURES)" --target $(AARCH64_TARGET)

aarch64-debug:
	cargo build --lib --features="$(FEATURES)" --target $(AARCH64_TARGET)
