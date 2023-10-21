all: release

release:
	cargo build --release
# objdump
	riscv32-unknown-elf-objdump -d target/riscv32i-unknown-none-elf/release/embedded_blossom > target/riscv32i-unknown-none-elf/release/embedded_blossom.s
# strip
	riscv32-unknown-elf-strip target/riscv32i-unknown-none-elf/release/embedded_blossom
# objcopy
	riscv32-unknown-elf-objcopy -O binary target/riscv32i-unknown-none-elf/release/embedded_blossom target/riscv32i-unknown-none-elf/release/embedded_blossom.bin

debug:
	cargo build
# objdump
	riscv32-unknown-elf-objdump -d target/riscv32i-unknown-none-elf/debug/embedded_blossom > target/riscv32i-unknown-none-elf/debug/embedded_blossom.s
# objcopy
	riscv32-unknown-elf-objcopy -O binary target/riscv32i-unknown-none-elf/debug/embedded_blossom target/riscv32i-unknown-none-elf/debug/embedded_blossom.bin
