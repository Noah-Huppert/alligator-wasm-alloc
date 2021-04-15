# Alligator
Rust Web Assembly allocator.

# Table Of Contents
- [Overview](#overview)
- [Usage](#usage)
- [Development](#development)
  - [Targets](#targets)
  - [Running Hello World](#running-hello-world)
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

## Targets
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

## Running Hello World
A hello world program is provided which utilizes
Alligator as the Rust global allocator.

This program can be built as a Web Assembly program or
as a host binary. The host binary is only used for
debugging purposes, see [Debugging](#debugging).

To run the hello world program with Wasmtime:

```
make hello-world-run-wasm
```

This will automatically build the hello world if it is
not up to date, to build it manually run:

```
make hello-world-build-wasm
```

The resulting Web Assembly is output as
an `alligator.wasm` file.

## Debugging
Due to the lack of debugging support for Web Assembly
debugging is easier to do in a native binary format 
like that of 32-bit libc systems (See [Debugging Targets](#debugging-targets)).

To debug with lldb:

```
make hello-world-debug-host
```

Inside of lldb one can then debug the `alloc`
function, for instance, by running the command:

```
((lldb) breakpoint set --method alloc
# aka
((lldb) br s -M alloc
```

If you need to build the host binary run:

```
make hello-world-build-host
```

And if you need to run the host binary:

```
make hello-world-run-host
```

If debugging in Web Assembly is absolutely required
lldb can be used with wasmtime:

```
make hello-world-debug-wasm
```

## Fuzzing
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
is called the free Vector. Each free vector can store up to 256 free segment
indexes.

For free segments smaller than size class `3` (8B) not all possible free indexes
can be stored in this size. For size classes greater than `3` (8B) there will
always be extra space in these free vectors. (The idea of dynamically sized
stacks: so that there is no loss or waste of memory for smaller or bigger size
classes, will be implemented soon).
