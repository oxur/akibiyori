# Install Akibiyori

Akibiyori is not yet published to crates.io. Install the current local package from a checkout or an extracted `.crate` archive; the intended registry command is listed below as a future, untested command.

## Requirements

- Rust 1.95, the exercised toolchain and declared minimum.
- Cobalt 0.20.4 and Pagefind 1.5.2 for complete native site builds.

Akibiyori does not download Cobalt or Pagefind. Install the pinned tools from their supported sources, then check the commands that will actually resolve:

```sh
cobalt --version
pagefind --version
```

## Install a local package on your Cargo bin path

From the repository checkout, install the current candidate:

```sh
cargo install --path . --locked
```

Cargo puts installed executables in the selected installation root's `bin/` directory. By default that root is normally `$HOME/.cargo`, unless `--root`, `CARGO_INSTALL_ROOT`, Cargo configuration or `CARGO_HOME` selects another location. Check your shell's selected binary before installing or replacing it:

```sh
command -v akibiyori
akibiyori --version
cargo install --list
```

If `command -v` identifies a different installation, compare its version and Cargo's installed-package list before deciding which installation you intend to update. Do not use `--force` to resolve an unexplained conflict. Add the intended Cargo `bin/` directory to `PATH`, then verify:

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

After a registry release is announced, the intended pinned installation command is:

```sh
cargo install akibiyori --version 0.1.0 --locked
```

This registry command has not been tested and is unavailable until that release is published. Repository metadata identifies the intended project URL; it does not establish current repository or package availability.

## Use the installed CLI

Copy the complete `examples/starter-site/` directory from the checkout or extracted package into a new directory outside the Akibiyori checkout. Follow the [consumer quick start](quickstart/) from that copy. It builds with the installed `akibiyori` command and the actual Cobalt and Pagefind tools.

## Contributor builds

To work on Akibiyori itself, use the repository's real build recipes from the checkout. The [contributor quick start](quickstart/) covers project Docs, Observatory, the four-demo showcase and the external Botanist walkthrough. For source-only deterministic generation, `akibiyori generate` writes Cobalt source without running either native tool. See [configuration](configuration/) for inputs and [Build and maintain](maintenance/) for output ownership.
