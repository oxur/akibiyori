# Third-party notices

This project includes original and adapted material from the public sources listed below. Retain this file and the accompanying license texts when redistributing the package.

## Fumadocs

The reference specimen prose, spacious documentation shell conventions, and compiled reading stylesheet are based on Fumadocs at commit `0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef` by Fuma Nama and contributors. Source: [Fumadocs](https://github.com/fuma-nama/fumadocs/tree/0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef). The two adapted articles are `apps/docs/content/docs/(framework)/index.mdx` and `apps/docs/content/docs/(framework)/what-is-fumadocs.mdx`.

The CSS distributed as `theme/vendor/fumadocs-reference.css` is a derived version of the deployed Fumadocs stylesheet `3xiw_38akfqyl.css` (source SHA256 `fa1588367414ad9c098ba5524159b0a0229db477d83db6b66c8104b6934521bf`; public projection SHA256 `479f5f76e838eed60ff1e2277803399fd2c0c778e4122b04c8daebf14de25e69`). It retains the reading UI rules and removes the separate optional sponsor marquee stylesheet block. The local shell overrides and its selected DOM mapping are described in `theme/vendor/PROVENANCE.md`.

Fumadocs source is MIT licensed; the full upstream license is in `theme/vendor/licenses/fumadocs-MIT.txt`. The typography extension's license is in `fumadocs-typography-MIT.txt`. The code-rendering style source notice is in `rehype-code-MIT.txt`.

## Fonts

Geist is © 2024 The Geist Project Authors and JetBrains Mono is © 2020 The JetBrains Mono Project Authors. Both font families are distributed under the SIL Open Font License 1.1; retain `geist-OFL.txt` and `jetbrains-mono-OFL.txt`. The served CSS is a labeled Latin-only projection of the pinned Fumadocs font stylesheet and references only the included Latin WOFF2 files. The upstream stylesheet is retained as `reference-fonts-source.css`.

## Lucide

The selected inline shell and specimen icons are from Lucide (`lucide-icons/lucide`, tag `1.61.0`, commit `c29c276f16590cf017126671535ad8c792be64d8`): search, copy, text alignment, card/callout icons, `rocket`, `circle-help`, `book-open`, `chevrons-up-down`, and `panel-left`. Retain the full dual ISC and Feather MIT license text in `lucide-dual-license.txt`.

## Tailwind CSS animation utilities

The compiled reference stylesheet includes applicable output from `tailwindcss-animate` 1.0.7 by Jamie Kyle, licensed under MIT. Retain `tailwindcss-animate-MIT.txt`.

## Scalar public example source

`examples/fumadocs-reference/scalar.yaml` and `openapi.ts` are unmodified copies from Fumadocs' pinned Scalar example. The OpenAPI document declares MIT and links to Scalar's license; the retrieved full Scalar MIT text is retained in `scalar-MIT.txt`. The original Scalar files remain an unchanged source/provenance specimen; the generator does not execute the TypeScript file or fetch the YAML from a service. The Planets example reads the separately documented four-reference derivative in `examples/fumadocs-reference/planets-corrected.yaml`.

## Rust Markdown dependencies

The contract-description reader uses [pulldown-cmark 0.13.6](https://github.com/raphlinus/pulldown-cmark/tree/v0.13.6), MIT licensed; [Ammonia 4.2.4](https://github.com/rust-ammonia/ammonia/tree/4.2.4), dual licensed MIT OR Apache-2.0; and [Syntect 5.2.0](https://github.com/trishume/syntect/tree/v5.2.0), MIT licensed. The exact license texts are retained as `pulldown-cmark-MIT.txt`, `ammonia-MIT.txt`, `ammonia-Apache-2.0.txt`, and `syntect-MIT.txt` in this directory.

## Kubernetes API Extensions OpenAPI fixture

`examples/kubernetes/contracts/apiextensions-v1.json` is an unchanged copy from the Kubernetes repository at commit `00ca73359fdd9662e71af5a72ce9ff4e8ad5ed6f`, path `api/openapi-spec/v3/apis__apiextensions.k8s.io__v1_openapi.json` (blob `67187166f68ae7860385feaf0c3744065a199d3e`). The Kubernetes project and contributors retain their source attribution. The full Apache License 2.0 text from the repository root is included at `examples/kubernetes/upstream/LICENSE`. The copied OpenAPI document has not been modified; measured provenance and reader limits are documented in `docs/public-fixtures.md`.
