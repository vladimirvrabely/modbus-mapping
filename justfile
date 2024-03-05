default:
	@just --list

build:
	cargo +nightly fmt
	cargo clippy
	cargo build --workspace --examples --all-features

run-tcp-device:
	cargo run --example tcp-device --features="simulator"

run-tcp-client:
	cargo run --example tcp-client --features="simulator"

doc:
	RUSTDOCFLAGS="--enable-index-page -Zunstable-options" cargo +nightly doc --workspace --no-deps --all-features

