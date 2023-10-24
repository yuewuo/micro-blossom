all: hls

# learning from https://arewefpgayet.rs/

OPT_LEVEL ?= 2
HLS_FLAGS ?= -C opt-level=$(OPT_LEVEL) -C overflow-checks=off -C no-vectorize-loops -C target-cpu=generic -C panic=abort
#  -C llvm-args="--opaque-pointers=false"

hls:
	rustup toolchain add nightly-2021-05-03 --profile minimal
	rustup override set nightly-2021-05-03
	rustc --version --verbose
	cargo clean
	RUSTFLAGS="--emit=llvm-ir $(HLS_FLAGS)" cargo build --release
# cargo rustc --release -- --emit=llvm-ir -o $(HLS_FLAGS)
	rustup override unset
	cd target && bambu ./release/deps/hls_blossom-*.ll ./release/deps/micro_blossom_nostd-*.ll --top-fname=run_benchmark --compiler=I386_CLANG12 -O$(OPT_LEVEL) -v2
