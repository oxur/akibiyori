# Install

Akibiyori 0.1.0-rc1 is a pre-release candidate. Check whether that exact version is currently listed on crates.io before using the registry install command below; the local checkout and extracted-archive workflows remain available.

## Requirements

- Rust 1.95, the exercised toolchain and declared minimum.
- Cobalt 0.20.4 and Pagefind 1.5.2 for complete native site builds.

Akibiyori does not download Cobalt or Pagefind. Install Cobalt from its [official guide](https://cobalt-org.github.io/getting-started/) or with its pinned crate command:

```sh
cargo install cobalt-bin --version 0.20.4 --locked
```

Download the native Pagefind 1.5.2 archive for your platform from its [official release](https://github.com/pagefind/pagefind/releases/tag/v1.5.2), then check the commands that will actually resolve:

```sh
cobalt --version
pagefind --version
```

## Check for an existing CLI

Before updating an existing Akibiyori installation, inspect which executable your shell will run and what Cargo has recorded:

```sh
command -v akibiyori
akibiyori --version
cargo install --list
```

If `command -v` identifies a different installation, compare its version and Cargo's installed-package list before deciding which installation you intend to update. Do not use `--force` to resolve an unexplained conflict.

## Install a local package on your Cargo bin path

From the repository checkout, install the current candidate:

```sh
cargo install --path . --locked
```

To install from a .crate archive instead, extract it into a fresh directory and change into the extracted package root before running the same Cargo command:

```sh
mkdir -p target/candidate-source
tar -xzf /path/to/akibiyori-0.1.0-rc1.crate -C target/candidate-source
cd target/candidate-source/akibiyori-0.1.0-rc1
cargo install --path . --locked
```

The filename and extracted directory describe the 0.1.0-rc1 candidate. Local package checks do not establish registry availability or exercise a registry installation.

Cargo puts installed executables in the selected installation root's `bin/` directory. By default that root is normally `$HOME/.cargo`, unless `--root`, `CARGO_INSTALL_ROOT`, Cargo configuration or `CARGO_HOME` selects another location. Add the intended Cargo `bin/` directory to `PATH`, then verify:

```sh
command -v akibiyori
akibiyori --version
```

To try a candidate without changing your ordinary Cargo installation, choose a separate root and put its `bin/` first on `PATH`:

```sh
cargo install --path . --locked --root target/akibiyori-cargo-root
export PATH="$PWD/target/akibiyori-cargo-root/bin:$PATH"
command -v akibiyori
akibiyori --version
```

This isolated example is intended for a disposable checkout-local test. The root must be new or intentionally owned by this task.

After verifying that crates.io lists this exact pre-release, the pinned registry installation command is:

```sh
cargo install akibiyori --version '=0.1.0-rc1' --locked
```

This repository snapshot describes the command syntax; check the live registry listing before running it.

## Use the installed CLI

Copy the complete `examples/starter-site/` directory from the checkout or extracted package into a new directory outside the Akibiyori checkout. Follow the [consumer quick start](../quickstart/) from that copy. It builds with the installed `akibiyori` command and the actual Cobalt and Pagefind tools.

## Contributor builds

To work on Akibiyori itself, use the repository's real build recipes from the checkout. The [contributor quick start](../quickstart/) covers project Docs, Observatory, the four-demo showcase and the external Botanist walkthrough. For source-only deterministic generation, `akibiyori generate` writes Cobalt source without running either native tool. See [configuration](../configuration/) for inputs and [Build and maintain](../maintenance/) for output ownership.
