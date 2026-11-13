# Dependencies and support

Akibiyori builds with a pinned Rust dependency graph and two separately installed native tools. The maintainers and Akibiyori contributors review these inputs when preparing each release and when updating a tool or bundled resource.

## Supported build hosts

The 0.1.0 workflow is configured for Rust 1.95.0 on these GitHub-hosted runners:

| Runner | Rust target | Cobalt | Pagefind |
| --- | --- | --- | --- |
| macOS 15, Apple silicon | `aarch64-apple-darwin` | 0.20.4 | 1.5.2 |
| macOS 15, Intel | `x86_64-apple-darwin` | 0.20.4 | 1.5.2 |
| Ubuntu 24.04, ARM64 | `aarch64-unknown-linux-gnu` | 0.20.4 | 1.5.2 static musl binary |
| Ubuntu 24.04, x86_64 | `x86_64-unknown-linux-gnu` | 0.20.4 | 1.5.2 static musl binary |

The workflow checks the archive SHA-256 values before extracting the named Cobalt and Pagefind executables. It installs no runtime downloader or Node.js dependency. The 0.1.0 host matrix does not include Windows; Windows is not a tested support claim. A configured hosted workflow still needs an authorized repository and an actual passing run before its hosted result can be reported.

## Rust dependencies and security review

The direct Rust dependencies are declared in `Cargo.toml` and resolved by `Cargo.lock`. The generator uses Ammonia 4.2.5 for its fixed HTML allowlist, pulldown-cmark 0.13.6 for Markdown, Syntect 5.2.0 for bounded code highlighting, and yaml-rust2 0.13.0 for supported YAML input. Cargo Audit checks the locked Rust graph against the RustSec advisory database; it does not scan native executables, browser bundles or every possible use of a dependency.

The dated package-candidate scan used cargo-audit 0.23.0 and RustSec database commit `4ef2df8e806a65e2f1ecc14f56f51249062b23d6` (2026-11-13). It found no vulnerability in the 150-package graph. It reported two unmaintained transitive crates through Syntect: bincode 1.3.3 (RUSTSEC-2025-0141) and yaml-rust 0.4.5 (RUSTSEC-2024-0320). Akibiyori calls Syntect for syntax highlighting and does not directly call either crate; the supported YAML reader uses yaml-rust2. These warnings remain visible for maintainer review and are not suppressed or treated as resolved.

The earlier locked Ammonia 4.2.4 version appeared in RUSTSEC-2026-0533; the candidate updates that direct pin to upstream 4.2.5. The current Markdown sanitizer has a fixed explicit tag allowlist that excludes the advisory's raw-text gadget tags and removes SVG and MathML elements. This is a bounded applicability observation, not proof that every caller input is harmless or a substitute for keeping the dependency updated.

## Native tools and generated browser assets

Install Cobalt 0.20.4 and Pagefind 1.5.2 from their supported upstream releases as described in [Install](../install/). Akibiyori verifies the exact versions and does not download them. The CI workflow downloads the corresponding official archive for each runner, checks its recorded SHA-256, and extracts only the required executable.

Pagefind creates the site's local search JavaScript, CSS and WebAssembly data from the built HTML. The selected interface and browser/build components are recorded in the public source file `theme/vendor/pagefind-provenance.json`: Pagefind 1.5.2, Svelte 4.2.20, bcp-47 2.1.0, is-alphabetical 2.0.1, is-alphanumerical 2.0.1, is-decimal 2.0.1, adequate-little-templates 1.0.2 and mark.js 8.11.1. Akibiyori's own browser adapters are vanilla JavaScript under `theme/js/`. The selected browser dependency notices do not describe the Rust dependency graph and do not claim a complete source audit of all transitive WebAssembly code.

First-party and third-party license texts are retained in the package and emitted byte-for-byte into each generated Cobalt source tree under `licenses/akibiyori/`. The [complete third-party notice text](../../licenses/akibiyori/THIRD_PARTY_NOTICES.txt) and the corresponding `vendor/`, `pacific/` and `examples/` texts travel with the completed site. This preserves the generic `/licenses/` route for a site's own content. Authors remain responsible for records and notices for their own custom assets and themes.

## Refresh and release review

At each release, maintainers and Akibiyori contributors should:

1. Review `Cargo.lock` with the current RustSec database and inspect every vulnerability or warning, including its inverse dependency chain and production use.
2. Check upstream Rust release notes, Cobalt and Pagefind release pages, runner labels, archive names, archive digests and required archive members.
3. Review Pagefind's pinned source tag and lockfiles; verify package archive integrity, license text and provenance before updating any selected browser component.
4. Refresh the recorded source versions, notice bytes, workflow pins and public documentation together, preserving existing license text and attribution.
5. Run `make check`, `make example`, `make example-prefix`, `make docs`, `make docs-prefix`, and `cargo publish --dry-run --locked` on the clean candidate; inspect the generated notices in root and prefixed sites.

The workflow audits on pushes, pull requests and manual dispatch; it has no scheduled scan. Maintainers own the review at every dependency or release update. A passing local Docker build does not establish that GitHub-hosted CI has passed, and package dry-run success does not publish the crate. The registry command remains gated on its separately authorized release.
