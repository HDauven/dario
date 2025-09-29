WASM_TARGET ?= wasm32-unknown-unknown
FEATURES ?= ffi

all:
	@cargo build --release

wasm:
	@cargo build --release --target $(WASM_TARGET) --features $(FEATURES)

wasm-js:
	@cargo build --release --target $(WASM_TARGET) --features js

clippy:
	@cargo clippy -- -D warnings
	@cargo clippy --target $(WASM_TARGET) --features ffi -- -D warnings
	@cargo clippy --target $(WASM_TARGET) --features js -- -D warnings

doc:
	@cargo doc --release

.PHONY: all wasm wasm-js clippy doc
