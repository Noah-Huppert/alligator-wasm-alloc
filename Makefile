.PHONY: build run verbose-run debug clean

BUILD_TARGET ?= wasm32-wasi
BUILD_OUT_DIR ?= target
BUILD_OUT_WASM ?= ${BUILD_OUT_DIR}/${BUILD_TARGET}/debug/alligator.wasm

build:
	cargo build --target ${BUILD_TARGET}

run:
	wasmtime run ${BUILD_OUT_WASM}

verbose-run:
	make run WASMTIME_BACKTRACE_DETAILS=1

debug:
	lldb -- wasmtime -g ${BUILD_OUT_WASM}

clean:
	rm -rf ${BUILD_OUT_DIR}
