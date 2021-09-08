# Alligator
A real-time memory allocator built for web assembly, written in Rust.

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
  - [Time Complexity](#time-complexity)
  - [Memory Limit](#memory-limit)
  - [Size Classes](#size-classes)
  - [MiniPages](#minipages)
  - [MetaPage](#metapage)
  - [Big Allocation](#big-allocation)
  - [Life Cycle of an Allocation](#life-cycle-of-an-allocation)

# Overview
Alligator is a _real-time_ memory allocator built for WebAssembly, written in Rust.  Using Alligator is as simple as adding two lines of code to your project, see the [Usage](#usage) instructions for more.

**Why do I need a different allocator?**  
The default Rust allocator is the wrong tool for the job when it comes to WebAssembly.

WebAssembly's memory model is very different from native platforms. The WebAssembly heap is a contiguous segment of memory which can only grow, with a maximum size of 4 GB([†](https://webassembly.github.io/spec/js-api/index.html#limits)).

The default Rust allocator was written for native platforms where the memory model's requirements are very different from WebAssembly's. Native allocators must deal with being able to allocate a seemingly infinite amount of memory, which is not contiguous, and which can be freed back to the operating system.

As a result the Rust allocator must make design trade-offs in order to accommodate these broad requirements. The Alligator WASM Allocator was designed from the start for WebAssembly's memory model. It does not need to contend with traditional native memory's requirements, allowing for optimizations.

See [Design](#design) for more details on performance and internal workings.  

[Contributors](./CONTRIBUTORS.md)

# Usage
Currently a crate has not been published as the project
is in development. However the `main` branch of the Git
repository is always stable, and releases are
tagged.

When a crate has been published the `AlligatorAlloc`
struct, which implements the
[`GlobalAlloc`](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)
trait, can be used via the
`#[global_allocator]` annotation:

```rust
// Tell Rust we want to use Alligator as the heap allocator.
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
  - Ex: `make liballigatorc-fuzz`
- `bench` - Example programs which use Alligator
  - The exact benchmark program must be specified using the `BENCH` variable (defaults to `use-global`), see [Running Benchmarks](#running-benchmarks) for details.
  - Actions:
    - `build` - Build target binary
	- `run` - `build` then run target binary
	- `debug` - `build` then use lldb to debug the binary
  - Targets:
    - `host` - 32-bit LibC platform, set via `TARGET_LIBC32`, defaults to `i686-unknown-linux-gnu`
	- `wasm` - WASI32 WASM
  - Ex: `make bench-run-host` or `bench-debug-wasm`
- `c-test` - Very basic C test program for `liballigatorc`
  - `c-test-build` - Build `c-test` Binary from `c-test.c`
  
Cargo is used to build the C dynamic library in `liballigatorc` and the binaries in `bench`. A host C++ toolchain is used to build AFL and Hangover fuzzer in `liballigatorc` and the test program in `c-test`.

To provide arguments to Cargo when it is building or running, modify the `CARGO_BARGS` (build arguments) and `RARGS` (run arguments) environment variables. Use `+=` when setting them to preserve behavior.
  
## Compile Targets
Alligator is meant as a heap allocator for WebAssembly
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
- `random-report`: Performs random allocations and outputs results as CSV rows (Requires you provide `CARGO_BARGS+=--features=metrics` to Make)

Specify which benchmark to run via the `BENCH` environment variable in Make (ex., in the command line specify `BENCH=<benchmark name>` like so `make bench-run-wasm BENCH=alloc-all`).

This program can be built as a WebAssembly program or as a host binary. The host binary is only used for debugging purposes, see [Debugging](#debugging).

To run a benchmark program with Wasmtime:

```
make bench-run-wasm BENCH=use-global
```

This will automatically build a benchmark if it is not up to date, to build it manually run:

```
make bench-build-wasm BENCH=use-global
```

The resulting WebAssembly is output as
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
Due to the lack of debugging support for WebAssembly
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

If debugging in WebAssembly is absolutely required
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
Alligator attempts to perform allocations and de-allocations of memory in constant time, with the goal of being well suited for real time WASM applications.

## Time Complexity
Allocations and de-allocations for under 2 KB of memory are constant time. This is done using [MiniPages](#minipages). Allocations and de-allocations above this size use [Big Allocation](#big-allocation) and are linear time.

The maximum size for constant time memory operations is constrained by the maximum size of a MiniPage. This size was chosen to try and pick a size which encompasses most allocations. The allocator is written so that this size can be changed via constant variables.

## Memory Limit
This allocator is designed to allocate no more than 4 GB of memory. This is due to limits set by the WASM specification:

> The maximum number of pages of a memory is 65536.

[WASM Specification JS implementation limits section](https://webassembly.github.io/spec/js-api/index.html#webassemblymemory-constructor).

Alligator has constants which set the maximum size:

- `MAX_HOST_PAGES`

TODO: MAX_HOST_PAGES is currently incorrectly set to `200`.

## Size Classes
Alligator is a size class allocator. Allocated objects are put into size class buckets. Size classes buckets are in power of two increments of bytes.

The smallest size class is `3` aka `2^3 = 8 bytes`. Smaller allocations will use this minimum size class. The largest size class is `11` aka `2^11 = 2048 bytes`. Larger allocations will use [Big Allocation](#big-allocation).

## MiniPages
For allocations smaller than the maximum size class of `11` (`2^11 = 2048 bytes`) the MiniPage allocation technique is used.

Alligator implements a less complex version of MiniHeaps and free Vectors from the [MESH allocator whitepaper](https://raw.githubusercontent.com/plasma-umass/Mesh/master/mesh-pldi19-powers.pdf). To avoid confusion between the two (as Alligator does not implement much functionality from the MESH paper's MiniHeaps) these will be called MiniPages in alligator.

MiniPages are 2 kilobyte sections of memory, from which same size class allocations are made.

Since all objects in a MiniPage heap section will be the same size, we can refer to them by their index. These uniformly sized pieces of the MiniPage memory section will be called Segments.

Each MiniPage stores a small header within the heap right before the 2kB memory section. This header contains: 

- Size class (1B)
- Bit-packed Segment free list (256B)

In order to find free MiniPages and segments in constant time a set of stacks is used for each size class. Popping from one of these stacks returns the next free MiniPage pointer or segment index. When MiniPages or segments are freed the allocator pushes onto these stacks. There is a stack for MiniPages and segments for each size class, which can hold `2^n` items (`n` = size class).

## MetaPage
The first bit of the heap is used to store metadata about the allocator state. This area is called the MetaPage. It will be lazily allocated.

It holds the free MiniPage and segment stacks mentioned in the [MiniPages](#minipages) section. As well as any metrics if the `metrics` feature is enabled.

## Big Allocation
For allocations larger than the maximum size class of `11` (`2^11 = 2048 bytes`) the big allocation technique is used.

Big allocation's free list is a linked list of `BigAllocHeader`s embedded in the heap. Segments of memory are allocated in ~2 kilobyte intervals (precise interval is the size of a `MiniPageHeader` plus 2 kilobytes). This is crucial for compatibility with MiniPage logic.

Once a big allocation segment has been de-allocated the underlying heap memory does not get returned to the host. Instead the big allocation segment is marked as free, and can be re-used in future big allocations.

Big allocations and de-allocations are O(n) via a linear search on the free linked list (`n` = number of big allocation items in the free linked list). Allocations will always try to use an existing free big allocation node using a first fit policy.

MiniPages are not used for these allocations because MiniPage logic cannot accommodate allocations larger than 2 kilobytes. Additionally MiniPage logic relies on constant MiniPage size, allowing pointer math to used find MiniPage headers in the heap without any searching. If MiniPages of different sizes were created for big allocations logic used for normal MiniPage allocations would break. Big allocations are provisioned in intervals of ~2 kilobytes for the same reason.

## Life Cycle of an Allocation
This presentation provides a rough outline of the design components working together. It is not currently up to date.

[Alligator Life Cycle Presentation](./docs/alligator-life-cycle-presentation.pdf)
