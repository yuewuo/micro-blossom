test_parameter ?= 

all: test coverage check build

fmt:
	cargo fmt --check

# A collection of lints to catch common mistakes and improve your Rust code.
clippy:
	cargo clippy -- -Dwarnings

clean:
	cargo clean

clean-env: clean

# make test 
# make test test_parameter=--silent -j8
test: test-parallel test-involves-scala

test-involves-scala:
	cargo test
	cargo test --release
# these tests are too slow, thus enable syndrome printing to help locate problem if they occur
	cargo run --release -- test embedded-scala --print-syndrome-pattern --print-command
	cargo run --release -- test embedded-looper --print-syndrome-pattern --print-command
	cargo run --release -- test embedded-axi4 --print-syndrome-pattern --print-command

# make test-parallel
# DISABLE_PROGRESS_BAR=1 make test-parallel -j8
test-parallel: test-primal-embedded test-dual-comb test-embedded-comb test-embedded-comb-pre-matching test-embedded-comb-layer-fusion test-embedded-comb-pre-matching-layer-fusion
# DISABLE_PROGRESS_BAR=1 make test-all-comb -j4
test-all-comb: test-dual-comb test-embedded-comb test-embedded-comb-pre-matching test-embedded-comb-layer-fusion

test-primal-embedded:
	cargo run --release -- test primal-embedded $(test_parameter)
test-dual-comb:
	cargo run --release -- test dual-comb $(test_parameter)
test-embedded-comb:
	cargo run --release -- test embedded-comb $(test_parameter)
test-embedded-comb-pre-matching:
	cargo run --release -- test embedded-comb-pre-matching $(test_parameter)
test-embedded-comb-layer-fusion:
	cargo run --release -- test embedded-comb-layer-fusion $(test_parameter)
test-embedded-comb-pre-matching-layer-fusion:
	cargo run --release -- test embedded-comb-pre-matching-layer-fusion $(test_parameter)

build: clean-env
	cargo test --no-run
	cargo test --no-run --release
	cargo build
	cargo build --release

check: clean-env
	cargo check
	cargo check --release

# test code coverage: see https://lib.rs/crates/cargo-llvm-cov
# run in release mode to include the randomized test cases
coverage:
	cargo llvm-cov --release --html
	# open target/llvm-cov/html/index.html


TEST_AXI4_COMMON = KEEP_RTL_FOLDER=1 EMBEDDED_BLOSSOM_MAIN=test_micro_blossom CONSIDER_CPU_TIME=false WITH_WAVEFORM=1 cargo run --release --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_d3.json
test-axi4:
# context depth
	CONTEXT_DEPTH=1 $(TEST_AXI4_COMMON)
	CONTEXT_DEPTH=2 $(TEST_AXI4_COMMON)
# clock divide by
	CLOCK_DIVIDE_BY=2 CONTEXT_DEPTH=1 $(TEST_AXI4_COMMON)
	CLOCK_DIVIDE_BY=3 CONTEXT_DEPTH=16 $(TEST_AXI4_COMMON)
# pipeline stages
	INJECT_REGISTERS=offload,execute,update $(TEST_AXI4_COMMON)
	INJECT_REGISTERS=offload,execute,update CONTEXT_DEPTH=2 $(TEST_AXI4_COMMON)
	INJECT_REGISTERS=offload,execute,update CLOCK_DIVIDE_BY=2 CONTEXT_DEPTH=2 $(TEST_AXI4_COMMON)
# broadcast and convergecast delay
	BROADCAST_DELAY=2 CONVERGECAST_DELAY=2 CONTEXT_DEPTH=1 $(TEST_AXI4_COMMON)
	BROADCAST_DELAY=3 CONVERGECAST_DELAY=4 CONTEXT_DEPTH=2 $(TEST_AXI4_COMMON)
	BROADCAST_DELAY=3 CONVERGECAST_DELAY=4 CLOCK_DIVIDE_BY=2 CONTEXT_DEPTH=2 $(TEST_AXI4_COMMON)
	BROADCAST_DELAY=3 CONVERGECAST_DELAY=4 CLOCK_DIVIDE_BY=3 $(TEST_AXI4_COMMON)
