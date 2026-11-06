# Akibiyori

[![][build-badge]][build]
[![][crate-badge]][crate]
[![][tag-badge]][tag]
[![][docs-badge]][docs]

[![][logo]][logo-large]

Akibiyori is a reusable Rust library and command line tool that turns supplied OpenAPI 3.0.x, 3.1.0 or 3.1.1 JSON or supported YAML contracts plus authored Markdown into Cobalt source pages and static sites. It is a structural documentation reader, not an OpenAPI validator or a business-payload generator. The [project docs and native demo hub](docs/index.md) are built from this checkout.

## Use the CLI

```sh
akibiyori generate --config site.json --output cobalt-source [--base-path /docs]
akibiyori build --config site.json --output build-root [--base-path /docs] \
  [--cobalt cobalt] [--pagefind pagefind]
```

A configuration contains version `1`, a site title, contract identifiers and relative JSON or supported JSON-compatible YAML paths, authored Markdown paths/titles/routes, and navigation links. An optional `sidebar_groups` list supplies grouped links for the reference shell; when omitted, existing navigation is shown in one Documentation group. An authored page may opt into a document header with `document: { "description": "...", "source_url": "https://..." }`. With `document`, the config page title is rendered as the header h1, so the body should not repeat it. Without `document`, the title is metadata only and the authored Markdown body supplies its own visible heading. Description is plain text, and a source URL must be an absolute HTTP(S) URL with a non-empty authority and no whitespace, control characters or backslashes. The page shows a direct source link and a Copy Markdown action containing the title, description and authored Markdown. Pages without `document` keep their authored body and breadcrumb behavior. Unknown fields and versions fail. Routes preserve logical spelling and case; generated pages use mapped `index.html` targets, including dotted identifiers. Output paths are checked against generated CSS, JavaScript, font, icon and Pagefind assets. Local references resolve against resources declared in the supplied input; logical document identifiers do not authorize network fetching. Unresolved or unsupported reference forms fail before output. Literal example/default/extension JSON remains data. Reusable parameter, request-body and response aliases are resolved with cycle checks and source pointers. Path Item Object `$ref` expansion is unsupported and fails before output. Caller titles, content, contracts, labels and links remain caller-owned data.

## Library

```rust
akibiyori::generate(config_path, output, base_path)
akibiyori::build(&BuildOptions::new(config_path, output))
```

`generate` returns a `GenerationReport` with authored, operation and model page counts plus presentation diagnostics. Inputs, route/reference validation and rendering finish before the first output write. Output must be absent or an empty real directory. A write-time filesystem failure can leave partial output.

`build` writes `build-root/source/` and `build-root/site/`, then runs the supplied Cobalt and Pagefind executables. With `--base-path /docs`, it indexes only the served `site/docs/` subtree and applies the prefix once to generated links. Executable paths containing directory components resolve from the caller's working directory before Cobalt changes directories; bare names resolve through `PATH`.

The build accepts Cobalt `0.20.4` and Pagefind `1.5.2`. Both versions and the fresh/empty output-root state are checked before source generation. Native failures can leave partial output; inspect it before removal. The build never deletes caller output and reports process stage, executable, status and stderr in typed errors.

Generated `/search/` uses the bundled Pagefind UI with a small local adapter. Article/reference content is indexed while site chrome and the search page are excluded. Schema fields have stable fragment IDs, allowing field-level search results. Operation/model indexes remain available without JavaScript. The reference-first shell has grouped navigation, a right contents rail, breadcrumbs, a skip link, keyboard-visible focus, bounded reading width and local overflow for long tables and code; no blanket accessibility-conformance claim is made.

Cobalt processes Liquid before Markdown. Generated and copied authored Markdown has complete frontmatter with `templated: false`; a complete leading authored frontmatter block is stripped, and an unterminated block fails. The caller owns the authored content and navigation data.

## Example

`examples/observatory/` is an independently authored illustrative catalog with two operations, nine named models and three Markdown pages. Its guide follows the observation response envelope through a selected variant, shared fields, an instrument and a calibration timestamp. The names and values are examples and make no claim about a live service.

`examples/petstore/` retains a pinned, unchanged public OpenAPI 3.0.4 Petstore contract with its Apache License 2.0 text and source provenance. Its guide follows `getPetById` into the `Pet` schema and searchable fields, with limits on non-JSON examples made explicit. It is a static reader example; no Petstore endpoint or authentication flow is executed.

`examples/fumadocs-reference.json` and `examples/fumadocs-reference/` provide a separate public specimen with adapted Quick Start and framework introduction prose, rich ordinary Markdown/HTML examples, and the Observatory contract as input. The upstream prose, compiled styles, fonts, icon and licenses are credited in `THIRD_PARTY_NOTICES.md` and `theme/vendor/PROVENANCE.md`. The Planets example renders a separately identified four-reference derivative of a pinned Scalar OpenAPI 3.1.1 document; the original source and TypeScript example remain unchanged for provenance.

## Development

Rust 1.95 is the exercised toolchain and declared `rust-version`; no lower MSRV is claimed. The lockfile is committed. The package has no Node, npm, Python, Go, plugin framework, network lookup or async-runtime dependency.

- `make compile` checks the Rust package and creates the initial lockfile if needed.
- `make check` runs formatting, locked check/clippy, docs and tests that invoke actual Cobalt and Pagefind. Set `COBALT` and `PAGEFIND` to installed executable paths when they are not on `PATH`.
- `make example` builds the root example under `target/example/observatory-build/{source,site}`.
- `make example-prefix` builds it beneath `/akibiyori-test` under `target/example/observatory-prefix-build/{source,site/akibiyori-test}`.
- `make reference-example` builds the separate specimen under `target/example/fumadocs-reference-build/{source,site}`.
- `make reference-example-prefix` builds it beneath `/fumadocs-reference` under `target/example/fumadocs-reference-prefix-build/{source,site/fumadocs-reference}`.
- `make petstore-example` builds the pinned Petstore fixture under `target/example/petstore-build/{source,site}`; `make petstore-example-prefix` builds it beneath `/petstore-reference` under `target/example/petstore-prefix-build/{source,site/petstore-reference}`.
- `make docs` and `make docs-prefix` build the project guide independently. `make demo-site` and `make demo-site-prefix` assemble those docs with the Observatory, Planets, Kubernetes, and Petstore sites while keeping their Pagefind indexes separate.
- Example recipes remove prior output only when an adjacent ownership marker exists and the output is still a real directory. A failed partial build has no marker and needs inspection before manual cleanup.
- `make clean-example` removes only marked example outputs.

Cobalt and Pagefind are supplied by the build-tool owner; Akibiyori does not download them. The Pagefind 1.5.2 UI, CSS, JavaScript runtime and WASM data are generated from served pages by the supplied executable. `theme/js/akibiyori-search.js` and `theme/js/akibiyori-reading.js` are the handwritten browser adapters maintained with the native output contract. Search is local to the generated index and makes no API calls. The bounded asset inventory is not a vulnerability-free or zero-JavaScript claim.

## Scope limits

All named component schemas receive pages, but this renderer does not claim full OpenAPI conformance or complete interpretation of every extension, callback, encoding, discriminator or JSON Schema dialect feature. Unsupported schema keywords receive diagnostics with source pointers and remain visible in retained JSON. Unsupported operation features similarly receive diagnostics and retained source details. Contract examples are labeled and are not asserted valid. Root and prefixed demo builds are framework examples, not production site integration, comprehensive accessibility certification or full framework parity.

[//]: ---Named-Links---

[logo]: assets/images/v1-banner-y250.png
[logo-large]: assets/images/v1-banner.png
[build]: https://github.com/oxur/akibiyori/actions/workflows/ci.yml
[build-badge]: https://github.com/oxur/akibiyori/actions/workflows/ci.yml/badge.svg
[crate]: https://crates.io/crates/akibiyori
[crate-badge]: https://img.shields.io/crates/v/akibiyori.svg
[docs]: https://docs.rs/akibiyori/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
[tag-badge]: https://img.shields.io/github/tag/oxur/akibiyori.svg
[tag]: https://github.com/oxur/akibiyori/tags

See [the favicon and home-screen icon pack](docs/icons.md) for local icon generation, optional configuration, and shared head tags.
