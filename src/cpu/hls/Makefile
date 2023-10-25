all: hls

# learning from https://arewefpgayet.rs/

OPT_LEVEL ?= 3
HLS_FLAGS ?= -C opt-level=$(OPT_LEVEL) -C overflow-checks=off -C no-vectorize-loops -C target-cpu=generic -C panic=abort -C link-arg=-nostartfiles
#  -C llvm-args="--opaque-pointers=false"

RUSTUP_VERSION ?= nightly-2021-05-03
# RUSTUP_VERSION ?= 1.52.0

hls:
	rustup toolchain add $(RUSTUP_VERSION) --profile minimal
	rustup override set $(RUSTUP_VERSION)
	rustc --version --verbose
	cargo clean
	RUSTFLAGS="--emit=llvm-ir $(HLS_FLAGS)" cargo build --release
	rustup override unset
	cd target && bambu ./release/deps/*.ll --top-fname=run_benchmark --compiler=I386_CLANG12 -O$(OPT_LEVEL) -v2
