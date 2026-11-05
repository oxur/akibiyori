# Third-party notices

This project includes original and adapted material from the public sources listed below. Retain this file and the accompanying license texts when redistributing the package.

## Fumadocs

The reference specimen prose, spacious documentation shell conventions, and compiled reading stylesheet are based on Fumadocs at commit `0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef` by Fuma Nama and contributors. Source: [Fumadocs](https://github.com/fuma-nama/fumadocs/tree/0cd7bd4a0b7a1fa911e61bbce1242695b1c9e9ef). The two adapted articles are `apps/docs/content/docs/(framework)/index.mdx` and `apps/docs/content/docs/(framework)/what-is-fumadocs.mdx`.

The CSS distributed as `theme/vendor/fumadocs-reference.css` is a derived version of the deployed Fumadocs stylesheet `3xiw_38akfqyl.css` (source SHA256 `fa1588367414ad9c098ba5524159b0a0229db477d83db6b66c8104b6934521bf`; public projection SHA256 `479f5f76e838eed60ff1e2277803399fd2c0c778e4122b04c8daebf14de25e69`). It retains the reading UI rules and removes the separate optional sponsor marquee stylesheet block. The local shell overrides and its selected DOM mapping are described in `theme/vendor/PROVENANCE.md`.

Fumadocs source is MIT licensed; the full upstream license is in `theme/vendor/licenses/fumadocs-MIT.txt`. The typography extension's license is in `fumadocs-typography-MIT.txt`. The code-rendering style source notice is in `rehype-code-MIT.txt`.

## Fonts

Geist is © 2024 The Geist Project Authors and JetBrains Mono is © 2020 The JetBrains Mono Project Authors. Both font families are distributed under the SIL Open Font License 1.1; retain `geist-OFL.txt` and `jetbrains-mono-OFL.txt`. The served CSS is a labeled Latin-only projection of the pinned Fumadocs font stylesheet and references only the included Latin WOFF2 files. The upstream stylesheet is retained as `reference-fonts-source.css`.

## Lucide

The search icon is from Lucide (`lucide-icons/lucide`, tag `1.61.0`, commit `c29c276f16590cf017126671535ad8c792be64d8`). Retain the full dual ISC and Feather MIT license text in `lucide-dual-license.txt`.

## Tailwind CSS animation utilities

The compiled reference stylesheet includes applicable output from `tailwindcss-animate` 1.0.7 by Jamie Kyle, licensed under MIT. Retain `tailwindcss-animate-MIT.txt`.

## Scalar public example source

`examples/fumadocs-reference/scalar.yaml` and `openapi.ts` are unmodified copies from Fumadocs' pinned Scalar example. The OpenAPI document declares MIT and links to Scalar's license; the retrieved full Scalar MIT text is retained in `scalar-MIT.txt`. These files are a source/provenance specimen only; the generator does not execute the TypeScript file, fetch the YAML from a service, or render this OpenAPI 3.1 document.
