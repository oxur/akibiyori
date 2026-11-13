# Package and release checks

Akibiyori's Cargo package contains the CLI, reusable Rust library, bundled layouts, styles, JavaScript adapters, fonts, built-in theme families, notices, and the public examples used by its embedded resources. The package deliberately excludes repository-only `AGENTS.md`, `.github` workflow material, and ignored image drafts. Cargo's normal ignore rules also apply; inspect the actual archive whenever that policy changes.

## Check a local package

Run these commands from a clean committed candidate with the supported Rust toolchain:

```sh
cargo package --locked
cargo publish --dry-run --locked
make package-check
```

`cargo package` verifies a pristine extracted build by default. The package consumer test extracts the resulting `.crate`, installs it under a temporary Cargo root, and builds a neutral external site with the installed `akibiyori` plus Cobalt 0.20.4 and Pagefind 1.5.2. It also compiles an independent Rust library consumer from the extracted package. It does not publish or install into the default Cargo root.

The repository's current installation guide describes a source checkout. Before a registry release, the registry install command is intended to be `cargo install akibiyori --version 0.1.0 --locked`; that command has not been registry-tested and should not be treated as an available release until publication is announced. Cobalt and Pagefind are separate native tools and are not downloaded by Akibiyori.

## Release authority

Package and dry-run success do not publish a package. Registry upload, GitHub repository configuration, hosted CI dispatch, tags, and public announcements require their separately authorized release steps. Keep first-party copyright ownership and third-party notices accurate in the final archive.
