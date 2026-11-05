# Akibiyori

Akibiyori is a reusable Rust library and command line tool that turns supplied OpenAPI 3.0.x JSON contracts and authored Markdown into deterministic Cobalt source pages. It is a structural documentation interpreter, not an OpenAPI validator or a business-payload generator.

## Use the CLI

```sh
akibiyori generate --config site.json --output cobalt-source [--base-path /docs]
akibiyori build --config site.json --output build-root [--base-path /docs] \
  [--cobalt cobalt] [--pagefind pagefind]
```

A configuration contains version `1`, a site title, contract identifiers and relative JSON paths, authored Markdown paths/titles/routes, and navigation links. Unknown fields and versions fail. Routes preserve logical spelling and case; generated pages use mapped `index.html` targets, including dotted identifiers. Output paths are checked against generated CSS, JavaScript and Pagefind assets. Local structural JSON Pointer references are checked against the original contract while literal example/default/extension JSON remains data. External references fail. Reusable parameter, request-body and response aliases are resolved with cycle checks and source pointers. Path Item Object `$ref` expansion is unsupported and fails before output. Caller titles, content, contracts, labels and links remain caller-owned data.

## Library

```rust
akibiyori::generate(config_path, output, base_path)
akibiyori::build(&BuildOptions::new(config_path, output))
```

`generate` returns a `GenerationReport` with authored, operation and model page counts plus presentation diagnostics. Inputs, route/reference validation and rendering finish before the first output write. Output must be absent or an empty real directory. A write-time filesystem failure can leave partial output.

`build` writes `build-root/source/` and `build-root/site/`, then runs the supplied Cobalt and Pagefind executables. With `--base-path /docs`, it indexes only the served `site/docs/` subtree and applies the prefix once to generated links. Executable paths containing directory components resolve from the caller's working directory before Cobalt changes directories; bare names resolve through `PATH`.

The build accepts Cobalt `0.20.4` and Pagefind `1.5.2`. Both versions and the fresh/empty output-root state are checked before source generation. Native failures can leave partial output; inspect it before removal. The build never deletes caller output and reports process stage, executable, status and stderr in typed errors.

Generated `/search/` uses the bundled Pagefind UI with a small local adapter. Article/reference content is indexed while site chrome and the search page are excluded. Schema fields have stable fragment IDs, allowing field-level search results. Operation/model indexes remain available without JavaScript. The generic shell provides responsive navigation, breadcrumbs, a skip link, visible focus, bounded reading width and local overflow for long tables and code; no blanket accessibility-conformance claim is made.

Cobalt processes Liquid before Markdown. Generated and copied authored Markdown has complete frontmatter with `templated: false`; a complete leading authored frontmatter block is stripped, and an unterminated block fails. The caller owns the authored content and navigation data.

## Example

`examples/observatory/` is an independently authored illustrative catalog with two operations, nine named models and three Markdown pages. Its guide follows the observation response envelope through a selected variant, shared fields, an instrument and a calibration timestamp. The names and values are examples and make no claim about a live service.

## Development

Rust 1.95 is the exercised toolchain and declared `rust-version`; no lower MSRV is claimed. The lockfile is committed. The package has no Node, npm, Python, Go, plugin framework, network lookup or async-runtime dependency.

- `make compile` checks the Rust package and creates the initial lockfile if needed.
- `make check` runs formatting, locked check/clippy, docs and tests that invoke actual Cobalt and Pagefind. Set `COBALT` and `PAGEFIND` to installed executable paths when they are not on `PATH`.
- `make example` builds the root example under `target/example/observatory-build/{source,site}`.
- `make example-prefix` builds it beneath `/akibiyori-test` under `target/example/observatory-prefix-build/{source,site/akibiyori-test}`.
- Example recipes remove prior output only when an adjacent ownership marker exists and the output is still a real directory. A failed partial build has no marker and needs inspection before manual cleanup.
- `make clean-example` removes only marked example outputs.

Cobalt and Pagefind are supplied by the build-tool owner; Akibiyori does not download them. The Pagefind 1.5.2 UI, CSS, JavaScript runtime and WASM data are generated from served pages by the supplied executable. `theme/js/akibiyori-search.js` is the handwritten adapter and is maintained with the native output contract. Search is local to the generated index and makes no API calls. The bounded asset inventory is not a vulnerability-free or zero-JavaScript claim.

## Scope limits

All named component schemas receive pages, but this renderer does not claim full OpenAPI conformance or complete interpretation of every extension, callback, encoding, discriminator or JSON Schema dialect feature. Unsupported schema keywords receive diagnostics with source pointers and remain visible in retained JSON. Unsupported operation features similarly receive diagnostics and retained source details. Contract examples are labeled and are not asserted valid. Root and prefixed demo builds are framework examples, not production site integration, comprehensive accessibility certification or full framework parity.
