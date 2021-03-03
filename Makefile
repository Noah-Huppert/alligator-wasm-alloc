.PHONY: build run

BUILD_TARGET ?= wasm32-wasi
BUILD_OUT_WASM ?= target/${BUILD_TARGET}/debug/alligator.wasm

build:
	cargo build --target ${BUILD_TARGET}

run:
	wasmtime run ${BUILD_OUT_WASM}

verbose-run:
	make run WASMTIME_BACKTRACE_DETAILS=1

