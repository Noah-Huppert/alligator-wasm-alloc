# Alligator
Rust Web Assembly allocator.

# Table Of Contents
- [Overview](#overview)
- [Building](#building)
- [Running](#running)

# Overview
Rust source code is located in `src/`. It is transformed
into web assembly and run in wasmtime.

Currently the allocator implementation and benchmark
program are one in the same. They will be split soon.

# Building
[Rust](https://www.rust-lang.org/) with the `wasm32-wasi` target, [wasmtime](https://wasmtime.dev/), and [GNU Make](https://www.gnu.org/software/make/)
must be installed.

The output of the project is a Web Assembly program.

To build the Rust source code into Web Assembly run:

```
make build
```

The resulting Web Assembly is output as
an `alligator.wasm` file.

# Running
Currently the allocator implementation and benchmark are
the same program.

This program is compiled into Web Assembly and can be run
with wasmtime:

```
make run
```

