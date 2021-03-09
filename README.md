# Alligator
Rust Web Assembly allocator.

# Table Of Contents
- [Overview](#overview)
- [Development](#development)
  - [Building](#building)
  - [Running](#running)
  - [Debugging](#debugging)

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

# Development
[Rust](https://www.rust-lang.org/) with the `wasm32-wasi` target, [wasmtime](https://wasmtime.dev/), [LLDB](https://lldb.llvm.org/), and [GNU Make](https://www.gnu.org/software/make/)
must be installed.

## Building
The output of the project is a Web Assembly program.

To build the Rust source code into Web Assembly run:

```
make build-wasm
```

The resulting Web Assembly is output as
an `alligator.wasm` file.

## Running
Run the Web Assembly benchmark with wasmtime:

```
make run-wasm
```

## Debugging
Due to the lack of debugging support for web assembly
debugging is easier to do in a native binary format 
like that of x86 (or whatever platform you have). The
`HostHeap` struct abstracts a WASM heap, its
implementation changes based on the build target. On
WASM it just proxies the WASM calls. On libc targets
it uses `malloc`.

To build the host binary:

```
make build-host
```

To run this binary:

```
make run-host
```

To debug with lldb:

```
make debug-host
```

Inside of lldb one can then debug the `alloc`
function, for instance, by running the command:

```
((lldb) breakpoint set --method alloc
# aka
((lldb) br s -M alloc
```

If debugging in Web Assembly is absolutely required
lldb can be used with wasmtime:

```
make debug-wasm
```
