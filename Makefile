.PHONY: build-wasm build-host run-wasm run-host run-wasm-verbose debug-wasm clean

WASM_TARGET ?= wasm32-wasi
BUILD_OUT_DIR ?= target
BUILD_OUT_WASM ?= ${BUILD_OUT_DIR}/${WASM_TARGET}/debug/alligator.wasm
BUILD_OUT_HOST_BIN ?= ${BUILD_OUT_DIR}/debug/alligator

build-host:
	cargo build

build-wasm:
	cargo build --target ${WASM_TARGET}

run-wasm:
	wasmtime run ${BUILD_OUT_WASM}

run-host:
	./${BUILD_OUT_HOST_BIN}

run-wasm-verbose:
	make run-wasm WASMTIME_BACKTRACE_DETAILS=1

debug-wasm:
	lldb -- wasmtime -g ${BUILD_OUT_WASM}

debug-host:
	rust-lldb ${BUILD_OUT_HOST_BIN}

clean:
	rm -rf ${BUILD_OUT_DIR}
