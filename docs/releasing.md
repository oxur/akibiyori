# Package and release checks

Akibiyori's Cargo package contains the CLI, reusable Rust library, bundled layouts, styles, JavaScript adapters, fonts, built-in theme families, notices, and the public examples used by its embedded resources. The package deliberately excludes repository-only `AGENTS.md`, `.github` workflow material, and ignored image drafts. Cargo's normal ignore rules also apply; inspect the actual archive whenever that policy changes.

The compressed package has a 10 MB upper bound in the real package-consumer check. This keeps the shipped resource set deliberate while leaving room above the current archive; if a future first-party resource needs more space, review its necessity, licensing, and consumer cost before changing the bound.

## Check a local package

Run these commands from a clean committed candidate with the supported Rust toolchain:

```sh
cargo package --locked
cargo publish --dry-run --locked
make package-check
```

`cargo package` verifies a pristine extracted build by default. The package consumer test extracts the resulting `.crate`, installs it under a temporary Cargo root, and builds a neutral external site with the installed `akibiyori` plus Cobalt 0.20.4 and Pagefind 1.5.2. It also compiles an independent Rust library consumer from the extracted package. It does not publish or install into the default Cargo root.

The [installation guide](../install/) describes installing the current local package and checking Cargo's selected bin/PATH installation. Before a registry release, the intended command is `cargo install akibiyori --version 0.1.0 --locked`; that command has not been registry-tested and should not be treated as available until publication is announced. Cobalt and Pagefind are separate native tools and are not downloaded by Akibiyori.

## Release authority

Package and dry-run success do not publish a package. Registry upload, GitHub repository configuration, hosted CI dispatch, tags, and public announcements require their separately authorized release steps. Keep first-party copyright ownership and third-party notices accurate in the final archive.

## Native support and notice checks

The [Dependencies and support](../dependencies/) guide records the four configured macOS/Linux runner targets, Rust 1.95.0, Cobalt 0.20.4, Pagefind 1.5.2 and the release/update review steps. Check the actual workflow results for each hosted target before claiming CI success; a local build or package dry run is not a hosted run.

Every generated Cobalt source tree carries the current license and attribution text in `licenses/akibiyori/`. Package consumers should deploy the complete generated `site/` tree, including notices under the selected prefix. Review the full package archive and generated root/prefix sites when a bundled resource, dependency, native tool or notice changes. Registry publication remains a separate authorized release action.
