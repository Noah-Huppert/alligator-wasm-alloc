# Alligator
Rust Web Assembly allocator.

# Table Of Contents
- [Overview](#overview)
- [Usage](#usage)
- [Development](#development)
  - [Makefile](#makefile)
  - [Compile Targets](#compile-targets)
  - [Debugging Targets](#compile-targets)
  - [Running Benchmarks](#running-benchmarks)
  - [Compile Time Features](#compile-time-features)
  - [Debugging](#debugging)
  - [Fuzzing](#fuzzing)
- [Design](#design)

# Overview
Alligator is an effort to build a great Web Assembly
memory allocator for Rust. It is in the beginning
stages. The Alligator development's vision:

> The memory model of Web Assembly is different
> from the `malloc` world programmers have been
> working with for decades. Web Assembly's memory is a
> contiguous byte array which can never shrink.
> Alligator treats Web Assembly as a real time embedded
> environment by trying to maintain a low memory
> overhead and have a constant time complexity.

The groundwork is being laid out right now, there is
much to do. See the releases section for detailed
progress information. 

# Usage
Currently a crate has not been published as the project
is in development. However the `main` branch of the Git
repository is always stable, and releases are
tagged weekly.

When a crate has been published the `AlligatorAlloc`
struct, which implements the
[`GlobalAlloc`](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)
trait, can be used via the
`#[global_allocator]` annotation:

```rs
// Tell Rust we want to use Alligator as the
// heap allocator.
#[global_allocator]
static ALLOC: AlligatorAlloc = AlligatorAlloc::INIT;

fn main() {
	// ... The rest of your program, unchanged
}
```

# Development
[Rust](https://www.rust-lang.org/) with the `wasm32-wasi` target (and `i686-unknown-linux-gnu` for development purposes), [wasmtime](https://wasmtime.dev/), [LLDB](https://lldb.llvm.org/), and [GNU Make](https://www.gnu.org/software/make/)
must be installed.

## Makefile
A Makefile is provided which performs all development tasks in this repository. Make targets are named in the format `<object>-<action>-<target>`, although some do not specify a `<target>`.

Normal users of Alligator should use the `alligator` Cargo crate.

The Makefile provides the following objects:

- `liballigatorc` - Dynamic C library with Alligator allocator
  - Actions:
    - `build` - Build the dynamic C library and headers, AFL and Hangover fuzzer
	- `fuzz` - `build` then run AFL and Hangover fuzzers on dynamic C library
  - Only builds for the `host` 32-bit LibC platform, set via `TARGET_LIBC32`, defaults to `i686-unknown-linux-gnu`
  - Ex: `liballigatorc-fuzz`
- `bench` - Example programs which use Alligator
  - The exact benchmark program must be specified using the `BENCH` variable (defaults to `use-global`), see [Running Benchmarks](#running-benchmarks) for details.
  - Actions:
    - `build` - Build target binary
	- `run` - `build` then run target binary
	- `debug` - `build` then use lldb to debug the binary
  - Targets:
    - `host` - 32-bit LibC platform, set via `TARGET_LIBC32`, defaults to `i686-unknown-linux-gnu`
	- `wasm` - WASI32 WASM
  - Ex: `bench-run-host` or `bench-debug-wasm`
- `c-test` - Very basic C test program for `liballigatorc`
  - `c-test-build` - Build `c-test` Binary from `c-test.c`
  
Cargo is used to build the C dynamic library in `liballigatorc` and the binaries in `bench`. A host C++ toolchain is used to build AFL and Hangover fuzzer in `liballigatorc` and the test program in `c-test`.

To provide arguments to Cargo when it is building or running, modify the `CARGO_BARGS` (build arguments) and `RARGS` (run arguments) environment variables. Use `+=` when setting them to preserve behavior.
  
## Compile Targets
Alligator is meant as a heap allocator for Web Assembly
only. However other targets can run Alligator for debugging purposes.

If type `heap::HeapType` is not found in `src/alloc/heap.rs` then the current build platform is not supported.

### Debugging Targets
Targets which implement libc with 32 bit pointer length can be used for development purposes. By default the Rust `i686-unknown-linux-gnu` target is used for this purpose.

**How:** A shim uses `malloc` to simulate WASM memory.
Allowing Alligator to run on non WASM targets.

**Why:** This makes debugging and fuzzing possible.
However, never should anyone use Alligator as their
global allocator when targeting a C stdlib system.
See [Debugging](#debugging) for more.

## Running Benchmarks
A few programs are provided which utilizes Alligator:

- `use-global` (Default): Performs a few heap allocations using Alligator as the programs Global Allocator
- `alloc-all`: Performs more than one MiniPage's worth of allocations for each size class
- `alloc-report`: (Wip) Performs benchmarks over a series of allocation sizes and outputs CSV rows (`benchmarks-record.sh` part of this, but wip at the moment)

Specify which benchmark to run via the `BENCH` environment variable in Make (ex., in the command line specify `BENCH=<benchmark name>` like so `make bench-run-wasm BENCH=alloc-all`).

This program can be built as a Web Assembly program or as a host binary. The host binary is only used for debugging purposes, see [Debugging](#debugging).

To run a benchmark program with Wasmtime:

```
make bench-run-wasm BENCH=use-global
```

This will automatically build a benchmark if it is not up to date, to build it manually run:

```
make bench-build-wasm BENCH=use-global
```

The resulting Web Assembly is output as
an `alligator.wasm` file.

## Compile Time Features
Compile time features can be provided to Cargo when building Alligator. This can enable features at compile time (no runtime cost).

Available features:

- `metrics` - Record statistics about allocation process. Results recorded to the `AllocMetrics` struct, which can be retrieved via the `AlligatorAlloc::metrics()` method. Additionally some debug information about why an allocation may have failed is available via the `AlligatorAlloc::alloc_failure_cause()` method and the `AllocFail` enum.

Compile Alligator with features by specifying the `--features=<feature>` Cargo build option. If you are using Make specify via the Cargo build args variable `CARGO_BARGS`:

```
make your-make-target CARGO_BARGS+=--features=metrics
```

## Debugging
Due to the lack of debugging support for Web Assembly
debugging is easier to do in a native binary format 
like that of 32-bit libc systems (See [Debugging Targets](#debugging-targets)).

To debug with lldb:

```
make bench-debug-host BENCH=use-global
```

Inside of lldb one can then debug the `alloc`
function, for instance, by running the command:

```
((lldb) breakpoint set --method alloc
# aka
((lldb) br s -M alloc
```

Or on line 679 of the `src/alloc/mod.rs` file:
```
((lldb) br set -f src/alloc/mod.rs -l 679
```

If you need to build the host binary run:

```
make bench-build-host BENCH=use-global
```

And if you need to run the host binary:

```
make bench-run-host BENCH=use-global
```

If debugging in Web Assembly is absolutely required
lldb can be used with wasmtime:

```
make bench-debug-wasm BENCH=use-global
```

## Fuzzing
**Fuzzing is currently not working and needs to be fixed**  
Initially fuzzing was completed on a 64-bit platform using a 64-bit binary. Now the binary is 32-bits, however development still takes place on a 64-bit platform. AFL++ and Hangover need to be compiled to 32-bits. This will be completed eventually.

The [HangOver memory allocator fuzzer](https://github.com/emeryberger/hangover),
which utilizes [AFL++](https://github.com/AFLplusplus/AFLplusplus#building-and-installing-afl), 
is used to fuzz the Alligator memory allocator.

The source code for both of these dependencies is
provided as git submodules, to retrieve them run:

```
git submodule update --init
```

Then run:

```
make liballigatorc-fuzz
```

This will automatically build AFL and HangOver, build
Alligator as a dynamic library, and run the
fuzzing setup.

Results will be stored in
`hangover-fuzzer/afl_out/default`. This fuzz Make target
also sets one's CPU clock frequency scaling to
`performance` mode using the `cpufreqctl` script. It
should set it back to what it was before automatically.

If you want to build AFL manually run:

```
make afl-build
```

If you want to build HangOver manually run:

```
make hangover-fuzzer-build
```

If you want to build the dynamic library manually run:

```
make liballigatorc-build
```

The `test-c.c` file is a basic C program which simply
calls `alloc` and `dealloc` from the dynamic library.
This is meant to ensure the dynamic library is
functioning minimally correctly. Build this file
by running:

```
make c-test-build
```

Then run `./c-test`.

# Design
## Memory Limit
This allocator is designed to allocate no more than 2 GB of memory.

## Size Classes
Alligator is a size class allocator. Allocated objects are put into size class
buckets. Size classes buckets are in power of two increments of bytes. The
smallest size class is `0` aka `2^0 = 1B`. The largest size class is `11`
aka `2^11 = 2048B`.

## MiniPages
Alligator implements a less complex version of MiniHeaps and free Vectors from
the [MESH allocator whitepaper](https://raw.githubusercontent.com/plasma-umass/Mesh/master/mesh-pldi19-powers.pdf).
To avoid confusion between the two (as Alligator does not implement much 
functionality from the MESH paper's MiniHeaps) these will be called MiniPages
in alligator.

MiniPages are 2kB sections of memory, from which same size class allocations are
made.

Since all objects in a MiniPage heap section will be the same size, we can
refer to them by their index. These uniformly sized pieces of the MiniPage
memory section will be called Segments.

Each MiniPage stores a small header within the heap right before the 2kB memory
section. This header contains: 

- Size class (1B)
- Bit-packed Segment free list (256B)
  - Right now this is constantly sized
  - In the future I would like to make the size depend on the size class according to this formula
  `ceiling(2kB / (2^size_class))` bits. The number of bits is rounded up to the
  nearest increment of 8, as to align the free list on 1 byte addresses.

In order to find free Segment indexes in constant time, a stack of free segment
indexes will be maintained for the most recently used MiniPage of each
size class. This will be stored in the program's stack memory area. This stack
is called the free segments stack. Each free segments vector can store `2^n` items per size class.

## Life Cycle of an Allocation
The above sections describe core concepts in a vacuum, without context. This section aims to describe how core components work together to allocate and then free a segment of memory.

[Alligator Life Cycle Presentation](./docs/alligator-life-cycle-presentation.pdf)
