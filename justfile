default:
	@just --list

build:
	cargo +nightly fmt
	cargo clippy
	cargo build --workspace --examples --all-features

run-tcp-device:
	cargo run --example tcp-device --features="simulator"

run-tcp-client:
	cargo run --example tcp-client

run-rtu-device:
	cargo run --example rtu-device --features="simulator,serial"

run-rtu-client:
	cargo run --example rtu-client --features="serial"

doc:
	RUSTDOCFLAGS="--enable-index-page -Zunstable-options" cargo +nightly doc --workspace --no-deps --all-features

