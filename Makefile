.PHONY: build test lint native

build:
	cargo build --workspace

test:
	cargo test --workspace

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

native:
	cmake --preset windows-x64
	cmake --build --preset windows-x64-release
	ctest --preset windows-x64
