default:
	@just --list

build:
	cargo +nightly fmt
	cargo clippy
	cargo build --workspace --examples --all-features

run-tcp-device:
	cargo run --example tcp-device --features="examples,simulator"

run-tcp-client:
	cargo run --example tcp-client --features="examples"

run-rtu-device:
	cargo run --example rtu-device --features="examples,simulator,serial"

run-rtu-client:
	cargo run --example rtu-client --features="serial"

doc:
	RUSTDOCFLAGS="--enable-index-page -Zunstable-options" cargo +nightly doc --workspace --no-deps --all-features

