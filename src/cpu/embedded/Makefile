all: release

TARGET ?= riscv32imac-unknown-none-elf
# TARGET ?= riscv32i-unknown-none-elf

release:
	cargo build --release --target $(TARGET)
# objdump
	riscv32-unknown-elf-objdump -d target/$(TARGET)/release/embedded_blossom > target/$(TARGET)/release/embedded_blossom.s
# strip
	riscv32-unknown-elf-strip target/$(TARGET)/release/embedded_blossom
# objcopy
	riscv32-unknown-elf-objcopy -O binary target/$(TARGET)/release/embedded_blossom target/$(TARGET)/release/embedded_blossom.bin
	ls -al target/$(TARGET)/release/embedded_blossom.bin

debug:
	cargo build --target $(TARGET)
# objdump
	riscv32-unknown-elf-objdump -d target/$(TARGET)/debug/embedded_blossom > target/$(TARGET)/debug/embedded_blossom.s
# objcopy
	riscv32-unknown-elf-objcopy -O binary target/$(TARGET)/debug/embedded_blossom target/$(TARGET)/debug/embedded_blossom.bin
