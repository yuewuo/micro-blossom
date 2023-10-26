all: test coverage check build

fmt:
	cargo fmt --check

# A collection of lints to catch common mistakes and improve your Rust code.
clippy:
	cargo clippy -- -Dwarnings

clean:
	cargo clean

clean-env: clean fmt clippy

test: clean-env
	cargo test

	cargo run --release -- test dual-rtl
	cargo run --release -- test embedded primal

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
