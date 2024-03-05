default:
	@just --list

build:
	cargo +nightly fmt
	cargo clippy
	cargo build --workspace --examples --all-features

ex target:
	cargo run --example {{target}}

doc:
	RUSTDOCFLAGS="--enable-index-page -Zunstable-options" cargo +nightly doc --workspace --no-deps --all-features

