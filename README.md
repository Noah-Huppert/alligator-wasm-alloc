# Alligator
Rust Web Assembly allocator.

# Table Of Contents
- [Overview](#overview)
- [Building](#building)
- [Running](#running)

# Overview
Rust source code is located in `src/`. It is transformed
into web assembly using wasm-pack. The
`web-demo/index.html` file imports and executes this
web assembly.

Currently the allocator implementation and benchmark
program are one in the same. They will be split soon.

# Building
[Rust](https://www.rust-lang.org/), [wasm-pack](https://rustwasm.github.io/wasm-pack/), and [GNU Make](https://www.gnu.org/software/make/)
must be installed.

The output of the project is a Web Assembly program.

To build the Rust source code into Web Assembly run:

```
make build
```

The resulting Web Assembly, and some helper binding
JavaScript, is outputted in the `pkg/` directory.

# Running
Currently the allocator implementation and benchmark are
the same program.

This program is meant to run in a browser. 
`web-demo/` directory contains a small amount of HTML and
JavaScript necessary to import and invoke this program.
This code is meant to be served from an HTTP server who's
root is the same directory as this README file.

Run an HTTP server in the same directory as this README
file and access the `web-demo/index.html` file. A simple 
web server using Python's `http.server` module can be
run using:

```
make serve
```
