all: hls

# learning from https://arewefpgayet.rs/

OPT_LEVEL ?= 3
HLS_FLAGS ?= -C opt-level=$(OPT_LEVEL) -C overflow-checks=off -C no-vectorize-loops -C target-cpu=generic -C panic=abort -C llvm-args="--opaque-pointers=false"

hls:
	rustup override set 1.69.0
	rustc --version --verbose
	@echo "[info] the llvm version should be 14, 15 or 16, see https://llvm.org/docs/OpaquePointers.html"
	cargo rustc --release -- --emit=llvm-ir -o target/hls.ll $(HLS_FLAGS)
	rustup override unset
# cd target && bambu ./hls-c6f071981a4ddc02.ll --top-fname=min_max_rust_idiomatic --compiler=I386_CLANG12 -O$(OPT_LEVEL) -v2
	cd target && bambu ./hls-0c6a6d1b5205049f.ll --top-fname=run_benchmark --compiler=I386_CLANG12 -O$(OPT_LEVEL) -v2
