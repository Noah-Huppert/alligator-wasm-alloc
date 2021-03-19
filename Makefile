# Constants
MAKE ?= make

# ... Source files
SRC_FILES_CORE ?= $(wildcard src/alloc/**)
SRC_FILES_HELLO_WORLD ?= src/hello-world.rs ${SRC_FILES_CORE}

# ... Build outputs
BUILD_OUT ?= ./target

BUILD_DIR_WASM ?= wasm32-wasi
BUILD_DIR_HOST ?= debug

# ... ... liballigatorc
LIBALLIGATORC_BUILD_OUT ?= ${BUILD_OUT}/${BUILD_DIR_HOST}/liballigatorc.so
LIBALLIGATORC_HEADER_FILE ?= liballigatorc.h
LIBALLIGATORC_HEADER_OUT ?= ${BUILD_OUT}/${BUILD_DIR_HOST}/${LIBALLIGATORC_HEADER_FILE}

# ... ... fuzzing
HANGOVER_DIR ?= hangover-fuzzer
HANGOVER_BUILD_OUT ?= ${HANGOVER_DIR}/hangover
AFL_DIR ?= AFLplusplus
AFL_CXX ?= ${AFL_DIR}/afl-c++
AFL_FUZZ ?= ${AFL_DIR}/afl-fuzz

# ... ... hello world
HELLO_WORLD_BUILD_HOST_OUT ?= ${BUILD_OUT}/${BUILD_DIR_HOST}/hello-world
HELLO_WORLD_BUILD_WASM_OUT ?= ${BUILD_OUT}/${BUILD_DIR_WASM}/debug/hello-world.wasm

# Just make a C binary which calls the Alligator
# functions to ensure they work bare minimum.
# WIP
c-test-build:
	g++ -L./target/debug -lalligatorc -g -include./target/debug/liballigatorc.h c-test.c -o c

# Build the alligator C dynamic library, used to fuzz
liballigatorc-build: ${LIBALLIGATORC_BUILD_OUT}
${LIBALLIGATORC_BUILD_OUT}: src/clib.rs ${SRC_FILES_CORE} $(wildcard src/bin/**)
	cargo build --lib
	cargo run --bin generate-cheaders
	mv ${LIBALLIGATORC_HEADER_FILE} ${BUILD_OUT}/${BUILD_DIR_HOST}

afl-build: ${AFL_CXX}
${AFL_CXX}: $(wildcard ${AFL_DIR}/src/**)
	${MAKE} -C ${AFL_DIR} distrib

# Build the memory allocator fuzzer named HangOver
hangover-fuzzer-build: ${HANGOVER_BUILD_OUT}
${HANGOVER_BUILD_OUT}: ${HANGOVER_DIR}/hangover.cpp afl-build
	${AFL_CXX} \
		-std=c++14 -O0 -g \
		-L${BUILD_OUT}/${BUILD_DIR_HOST} -lalligatorc \
		-include${LIBALLIGATORC_HEADER_OUT} \
		-DHANGOVER_MALLOC=alligator_alloc \
		-DHANGOVER_FREE=alligator_dealloc \
		-DHANGOVER_REALLOC=alligator_realloc \
		${HANGOVER_DIR}/hangover.cpp -o ${HANGOVER_BUILD_OUT}

# Run the memory allocator fuzzer on the dynamic
# alligatorc library
liballigatorc-fuzz: afl-build hangover-fuzzer-build liballigatorc-build
	sudo ./cpufreqctl -p
	LD_LIBRARY_PATH=${BUILD_OUT}/${BUILD_DIR_HOST}:${LD_LIBRARY_PATH} \
	${AFL_FUZZ} \
		-m 18000000 \
		-t 100 \
		-x ${HANGOVER_DIR}/dictionary/malloc.dict \
		-i ${HANGOVER_DIR}/afl_in \
		-o ${HANGOVER_DIR}/afl_out \
		${HANGOVER_BUILD_OUT}
	sudo ./cpufreqctl -u

# Build hello world
hello-world-build-wasm: ${SRC_FILES_HELLO_WORLD}
	cargo build --bin hello-world --target wasm32-wasi
hello-world-build-host: ${SRC_FILES_HELLO_WORLD}
	cargo build --bin hello-world

# Run hello world
hello-world-run-wasm: hello-world-build-wasm
	wasmtime run ${HELLO_WORLD_WASM_OUT}
hello-world-run-host: hello-world-build-host
	./${HELLO_WORLD_BUILD_HOST_OUT}

# Debug hello world
hello-world-debug-wasm: hello-world-build-wasm
	lldb -- wasmtime run -g ${HELLO_WORLD_BUILD_WASM_OUT}
hello-world-debug-host: hello-world-build-host
	rust-lldb ./${HELLO_WORLD_BUILD_HOST_OUT}

# Remove build outputs
clean:
	rm -rf ${BUILD_OUT} || true
	${MAKE} -C ${HANGOVER_DIR} clean
	${MAKE} -C ${AFL_DIR} clean
