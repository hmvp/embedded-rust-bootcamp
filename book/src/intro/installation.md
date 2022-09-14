# Installing the tools

This page contains OS-agnostic installation instructions for a few of the tools:

## Rust Toolchain

Install rustup by following the instructions at [https://rustup.rs](https://rustup.rs).

**NOTE** Make sure you have a compiler version equal to or newer than `1.31`. `rustc
-V` should return a date newer than the one shown below.

``` text
$ rustc -V
rustc 1.31.1 (b6c32da9b 2018-12-18)
```

For bandwidth and disk usage concerns the default installation only supports
native compilation. To add cross compilation support for the ARM Cortex-M
architectures choose one of the following compilation targets. For the Raspberry Pico
board used for the examples in this book, use the `thumbv6m-none-eabi` target.

Cortex-M0, M0+, and M1 (ARMv6-M architecture):

``` console
rustup target add thumbv6m-none-eabi
```

## Other tools

### `cargo-binutils`

``` text
cargo install cargo-binutils

rustup component add llvm-tools-preview
```

### `flip-link`

``` text
cargo install flip-link
```

## Attribution

Some pages from this book are based upon the rust-embedded book found in [this repository] which is developed by the [resources team].
This page is loosely based on this [original page].

[this repository]: https://github.com/rust-embedded/book
[resources team]: https://github.com/rust-embedded/wg#the-resources-team
[original page]: https://docs.rust-embedded.org/book/intro/install.html
