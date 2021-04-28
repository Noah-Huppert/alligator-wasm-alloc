# Constants
MAKE ?= make

# ... Will be set when calling any benchmark target, defaults to use-global
BENCH ?= use-global

# ... Source files
SRC_FILES_CORE ?= $(wildcard src/alloc/**)
SRC_FILES_BENCH ?= src/bench-${BENCH}.rs ${SRC_FILES_CORE}

# ... Build outputs
BUILD_OUT ?= ./target

TARGET_WASM ?= wasm32-wasi
TARGET_LIBC32 ?= i686-unknown-linux-gnu

# ... ... liballigatorc
LIBALLIGATORC_LIB_OUT ?= ${BUILD_OUT}/${TARGET_LIBC32}/debug/liballigatorc.so
LIBALLIGATORC_HEADER_FILE ?= liballigatorc.h
LIBALLIGATORC_HEADER_OUT ?= ${BUILD_OUT}/${TARGET_LIBC32}/debug/${LIBALLIGATORC_HEADER_FILE}
LIBALLIGATORC_BUILD_OUT ?= ${LIBALLIGATORC_LIB_OUT} ${LIBALLIGATORC_HEADER_OUT}

# ... ... fuzzing
HANGOVER_DIR ?= hangover-fuzzer
HANGOVER_BUILD_OUT ?= ${HANGOVER_DIR}/hangover
AFL_DIR ?= AFLplusplus
AFL_CXX ?= ${AFL_DIR}/afl-c++
AFL_FUZZ ?= ${AFL_DIR}/afl-fuzz

# ... ... hello world
BENCH_BUILD_HOST_OUT ?= ${BUILD_OUT}/${TARGET_LIBC32}/debug/bench-${BENCH}
BENCH_BUILD_WASM_OUT ?= ${BUILD_OUT}/${TARGET_WASM}/debug/bench-${BENCH}.wasm

# Just make a C binary which calls the Alligator
# functions to ensure they work bare minimum.
# WIP
c-test-build:
	g++ -L./target/debug -lalligatorc -g -include./target/debug/liballigatorc.h c-test.c -o c-test

# Build the alligator C dynamic library, used to fuzz
liballigatorc-build: ${LIBALLIGATORC_BUILD_OUT}
${LIBALLIGATORC_BUILD_OUT}: src/clib.rs ${SRC_FILES_CORE} $(wildcard src/bin/**)
	cargo build --lib --target ${TARGET_LIBC32} ${CARGO_BARGS}
	cargo run --bin generate-cheaders
	mv ${LIBALLIGATORC_HEADER_FILE} ${LIBALLIGATORC_HEADER_OUT}

afl-build: ${AFL_CXX}
${AFL_CXX}: $(wildcard ${AFL_DIR}/src/**)
	${MAKE} -C ${AFL_DIR} distrib

# Build the memory allocator fuzzer named HangOver
hangover-fuzzer-build: ${HANGOVER_BUILD_OUT}
${HANGOVER_BUILD_OUT}: ${HANGOVER_DIR}/hangover.cpp afl-build
	${AFL_CXX} \
		-std=c++14 -O0 -g \
		-L${BUILD_OUT}/${TARGET_LIBC32}/debug -lalligatorc \
		-include${LIBALLIGATORC_HEADER_OUT} \
		-DHANGOVER_MALLOC=alligator_alloc \
		-DHANGOVER_FREE=alligator_dealloc \
		-DHANGOVER_REALLOC=alligator_realloc \
		${HANGOVER_DIR}/hangover.cpp -o ${HANGOVER_BUILD_OUT}

# Run the memory allocator fuzzer on the dynamic
# alligatorc library
liballigatorc-fuzz: liballigatorc-build afl-build hangover-fuzzer-build
	sudo ./cpufreqctl -p
	bash -c "trap 'sudo ./cpufreqctl -u' EXIT; \
	LD_LIBRARY_PATH=${BUILD_OUT}/${TARGET_LIBC32}:${LD_LIBRARY_PATH} \
	${AFL_FUZZ} \
		-m 18000000 \
		-t 100 \
		-x ${HANGOVER_DIR}/dictionary/malloc.dict \
		-i ${HANGOVER_DIR}/afl_in \
		-o ${HANGOVER_DIR}/afl_out \
		${HANGOVER_BUILD_OUT}"

# Build hello world
bench-build-wasm: ${SRC_FILES_BENCH}
	cargo build --bin bench-${BENCH} --target ${TARGET_WASM} ${CARGO_BARGS}
bench-build-host: ${SRC_FILES_BENCH}
	cargo build --bin bench-${BENCH} --target ${TARGET_LIBC32} ${CARGO_BARGS}

# Run hello world
bench-run-wasm: bench-build-wasm
	WASMTIME_BACKTRACE_DETAILS=1 wasmtime run ${BENCH_BUILD_WASM_OUT} ${RARGS}
bench-run-host: bench-build-host
	./${BENCH_BUILD_HOST_OUT} ${RARGS}

# Debug hello world
bench-debug-wasm: bench-build-wasm
	lldb -- wasmtime run -g ${BENCH_BUILD_WASM_OUT}
bench-debug-host: bench-build-host
	rust-lldb ./${BENCH_BUILD_HOST_OUT}

# Remove build outputs
clean:
	rm -rf ${BUILD_OUT} || true
	${MAKE} -C ${HANGOVER_DIR} clean
	${MAKE} -C ${AFL_DIR} clean
