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

# Overview
Alligator is an effort to build a great Web Assembly
memory allocator for Rust. It is in the beginning
stages. The Alligator development's vision:

> The memory model of Web Assembly is different
> from the `malloc` world programmers have been
> working with for decades. Web Assembly's memory is a
> contiguous byte array which can never shrink.
> Alligator treats Web Assembly as an embedded
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
[Rust](https://www.rust-lang.org/) with the `wasm32-wasi` target, [wasmtime](https://wasmtime.dev/), [LLDB](https://lldb.llvm.org/), and [GNU Make](https://www.gnu.org/software/make/)
must be installed.

## Targets
Alligator is meant as a heap allocator for Web Assembly
only. However targets which implement the C standard
library can run Alligator for debugging purposes.

### C stdlib
**Alligator on the C stdlib is only for development.**

Alligator can target a host binary or a dynamic library.
For example the hello world binary can be built as a
host binary instead of a `.wasm` file.

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
like that of x86 (or whatever platform you have). The
`HostHeap` struct abstracts a WASM heap. Its
implementation changes based on the build target. On
WASM it just proxies the WASM memory calls. On libc
targets it uses `malloc`.

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
