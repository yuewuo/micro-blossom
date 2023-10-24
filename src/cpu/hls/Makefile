all: hls


OPT_LEVEL ?= 2
HLS_FLAGS ?= -C opt-level=$(OPT_LEVEL) -C overflow-checks=off -C no-vectorize-loops -C target-cpu=generic -C panic=abort -C llvm-args="--opaque-pointers=false"

hls:
	mkdir -p target
	rustup override set 1.69.0
	rustc --version --verbose
	echo "the llvm version should be 14, 15 or 16, see https://llvm.org/docs/OpaquePointers.html"
	rustc --emit=llvm-ir --crate-type=lib src/main.rs -o target/hls.ll $(HLS_FLAGS)
	cd target && bambu ./hls.ll --top-fname=min_max_rust_idiomatic --compiler=I386_CLANG12 -O$(OPT_LEVEL) -v2
