# Install

Akibiyori is currently used from a Rust source checkout. This guide does not claim a crates.io release or a prebuilt binary.

## Requirements

- Rust 1.95, the exercised toolchain and the package's declared minimum.
- Cobalt 0.20.4 and Pagefind 1.5.2 on `PATH` for native site builds.
- Git and a local checkout of the public repository.

Akibiyori does not download Cobalt or Pagefind. Follow [Cobalt’s installation guide](https://cobalt-org.github.io/getting-started/) or install the pinned crate with `cargo install cobalt-bin --version 0.20.4 --locked`. Download the native Pagefind 1.5.2 archive for your platform from its [official release](https://github.com/pagefind/pagefind/releases/tag/v1.5.2). Check both tools before building:

```sh
cobalt --version
pagefind --version
```

## Install from the checkout

From the repository root:

```sh
cargo install --path . --locked --root "$HOME/.local"
"$HOME/.local/bin/akibiyori" --version
```

Add `$HOME/.local/bin` to `PATH` if you want to invoke `akibiyori` by name. For a one-off build without installing the binary, use `cargo run --locked -- build ...`; the [quick start](../quickstart/) shows a repository Make target that builds the included example with the actual CLI, Cobalt and Pagefind.

For a source-only deterministic generation, `akibiyori generate` writes Cobalt source but does not run either native tool. See [configuration](../configuration/) for the required inputs and [maintenance](../maintenance/) for the output-directory rules.
