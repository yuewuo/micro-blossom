all: release

# TARGET ?= riscv32imac-unknown-none-elf
TARGET ?= riscv32i-unknown-none-elf

release:
	cargo build --release --target $(TARGET)
# objdump
	riscv32-unknown-elf-objdump -d target/$(TARGET)/release/embedded_blossom_riscv > target/$(TARGET)/release/embedded_blossom_riscv.s
# strip
	riscv32-unknown-elf-strip target/$(TARGET)/release/embedded_blossom_riscv
# objcopy
	riscv32-unknown-elf-objcopy -O binary target/$(TARGET)/release/embedded_blossom_riscv target/$(TARGET)/release/embedded_blossom_riscv.bin
	ls -al target/$(TARGET)/release/embedded_blossom_riscv.bin

debug:
	cargo build --target $(TARGET)
# objdump
	riscv32-unknown-elf-objdump -d target/$(TARGET)/debug/embedded_blossom_riscv > target/$(TARGET)/debug/embedded_blossom_riscv.s
# objcopy
	riscv32-unknown-elf-objcopy -O binary target/$(TARGET)/debug/embedded_blossom_riscv target/$(TARGET)/debug/embedded_blossom_riscv.bin
