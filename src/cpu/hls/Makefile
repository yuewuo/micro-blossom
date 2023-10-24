all: hls

# learning from https://arewefpgayet.rs/

OPT_LEVEL ?= 3
HLS_FLAGS ?= -C opt-level=$(OPT_LEVEL) -C overflow-checks=off -C no-vectorize-loops -C target-cpu=generic -C panic=abort -C llvm-args="--opaque-pointers=false"

hls:
	rustup override set 1.69.0
	cargo rustc -- --version --verbose
	@echo "[info] the llvm version should be 14, 15 or 16, see https://llvm.org/docs/OpaquePointers.html"
	cargo rustc -- --emit=llvm-ir $(HLS_FLAGS)
	rustup override unset
# cd target && bambu ./hls-*.ll --top-fname=min_max_rust_idiomatic --compiler=I386_CLANG12 -O$(OPT_LEVEL) -v2
